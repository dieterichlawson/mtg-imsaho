//! The auto-tap planner against a brute-force oracle, over generated boards.
//!
//! `compute_autotap` picks which sources to tap for a cost, and its choice is
//! a heuristic that has been retuned at every turn (issues #84, #114, #252,
//! #615, #674, #678, #679). The unit tests in `mana.rs` pin its contract on
//! a dozen hand-picked boards; the three bugs fixed in commit 33dc369 were
//! all found by a throwaway crate that compared the planner with every
//! hand-tap over ~320k random boards and were on none of those dozen. This
//! file makes that comparison permanent.
//!
//! The oracle is "tapping by hand": pick any subset of the sources, one mana
//! ability each, activate the free ones and then the cost-bearing filters
//! (paying each filter's `{1}` out of the pool with any one mana), and see
//! whether what floats pays the cost. It is exhaustive — at most seven
//! sources with at most two Shimmering Grottos among them is a few thousand
//! plans — so a planner that gives up early, offers a plan the payment
//! cannot run, taps a source it did not need, or strands a spell in hand
//! that another plan would have kept castable is caught rather than trusted.
//!
//! The boards are seeded, so a failure names the seed and the board and is
//! reproducible; `AUTOTAP_BRUTE_FORCE_SEED` and `AUTOTAP_BRUTE_FORCE_BOARDS`
//! run a different sweep locally.

use std::collections::HashMap;

use mtg_engine::cards::ManaAbilityDef;
use mtg_engine::ids::ObjectId;
use mtg_engine::mana::{self, ManaSource, ManaSourceKind};
use mtg_engine::types::{Color, ManaCost, ManaPool, ManaSymbol, ManaType};

/// The sweep a plain `cargo test` runs. Seeded so the boards are the same
/// on every run, and sized to a few seconds of debug-build time: the oracle
/// enumerates every plan on every board, and that is where the time goes.
const DEFAULT_SEED: u64 = 0x5eed_a070_7a90_2026;
const DEFAULT_BOARDS: u64 = 2000;

/// A tap plan as the planner returns it: (source, ability index), in the
/// order the entries are activated.
type Plan = Vec<(ObjectId, usize)>;

// ---- a deterministic generator ----
//
// Written here rather than taken from `rand`: a test's boards must be the
// same on every machine and after every dependency bump, and a seed printed
// in a failure must rebuild the board that failed.

/// xorshift64*, seeded; good enough to spread boards, with no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // xorshift's one fixed point is zero.
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A number in `0..n`. The modulo bias is irrelevant at these sizes.
    fn below(&mut self, n: u64) -> u64 {
        (self.next() >> 32) % n
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[usize::try_from(self.below(items.len() as u64)).expect("a small index")]
    }
}

/// The per-board generator: mixing the sweep's seed with the board's index
/// means board `i` can be rebuilt on its own from the two numbers a failure
/// prints.
fn board_rng(seed: u64, index: u64) -> Rng {
    let mut r = Rng::new(seed ^ index.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    // Warm up so neighbouring indices do not start from neighbouring states.
    for _ in 0..4 {
        r.next();
    }
    r
}

// ---- the sources ----

/// The kinds of mana source the card pool has, as the planner sees them.
#[derive(Clone, Copy, Debug)]
enum SourceKind {
    /// A basic land: `{T}: Add` one colour.
    Basic(Color),
    /// A check land such as Clifftop Retreat: one free ability per colour.
    Dual(Color, Color),
    /// Sol Ring: `{T}: Add {C}{C}`.
    SolRing,
    /// Avacyn's Pilgrim and the like: a creature that taps for a colour.
    Creature(Color),
    /// Shimmering Grotto: `{T}: Add {C}` and `{1}, {T}: Add one mana of any
    /// color`, declared as five filter abilities the way the card does.
    Grotto,
    /// Deranged Assistant: `{T}, Mill a card: Add {C}` — a side effect.
    Assistant,
}

fn ability(index: usize, produced: Vec<(ManaType, u32)>, cost: ManaCost, side_effects: bool)
    -> ManaAbilityDef
{
    ManaAbilityDef {
        ability_index: index,
        description: format!("{cost}, {{T}}: Add {produced:?}"),
        produced,
        requires_tap: true,
        cost,
        has_side_effects: side_effects,
    }
}

fn free(index: usize, mana_type: ManaType) -> ManaAbilityDef {
    ability(index, vec![(mana_type, 1)], ManaCost::free(), false)
}

fn source(id: u64, kind: SourceKind) -> ManaSource {
    let (abilities, source_kind) = match kind {
        SourceKind::Basic(c) => (vec![free(0, c.into())], ManaSourceKind::BasicMana),
        SourceKind::Dual(a, b) => (vec![free(0, a.into()), free(1, b.into())], ManaSourceKind::NonBasicMana),
        SourceKind::SolRing => (
            vec![ability(0, vec![(ManaType::Colorless, 2)], ManaCost::free(), false)],
            ManaSourceKind::BasicMana,
        ),
        SourceKind::Creature(c) => (vec![free(0, c.into())], ManaSourceKind::Creature),
        SourceKind::Grotto => {
            // Shaped as `cards/isd/shimmering_grotto.rs` declares it.
            let mut abilities = vec![free(0, ManaType::Colorless)];
            for (i, c) in Color::ALL.into_iter().enumerate() {
                abilities.push(ability(i + 1, vec![(c.into(), 1)],
                    ManaCost::new(vec![ManaSymbol::Generic(1)]), false));
            }
            (abilities, ManaSourceKind::NonBasicMana)
        }
        SourceKind::Assistant => (
            vec![ability(0, vec![(ManaType::Colorless, 1)], ManaCost::free(), true)],
            ManaSourceKind::HasSideEffects,
        ),
    };
    ManaSource { object_id: ObjectId(id), abilities, source_kind }
}

fn letter(c: Color) -> char {
    match c {
        Color::White => 'W',
        Color::Blue => 'U',
        Color::Black => 'B',
        Color::Red => 'R',
        Color::Green => 'G',
    }
}

fn kind_name(kind: SourceKind) -> String {
    match kind {
        SourceKind::Basic(c) => match c {
            Color::White => "Plains",
            Color::Blue => "Island",
            Color::Black => "Swamp",
            Color::Red => "Mountain",
            Color::Green => "Forest",
        }.to_string(),
        SourceKind::Dual(a, b) => format!("Dual({}/{})", letter(a), letter(b)),
        SourceKind::SolRing => "Sol Ring".to_string(),
        SourceKind::Creature(c) => format!("Pilgrim({})", letter(c)),
        SourceKind::Grotto => "Grotto".to_string(),
        SourceKind::Assistant => "Assistant".to_string(),
    }
}

/// The colours a kind can make, for biasing costs toward what the board has.
fn kind_colors(kind: SourceKind) -> Vec<Color> {
    match kind {
        SourceKind::Basic(c) | SourceKind::Creature(c) => vec![c],
        SourceKind::Dual(a, b) => vec![a, b],
        SourceKind::Grotto => Color::ALL.to_vec(),
        SourceKind::SolRing | SourceKind::Assistant => vec![],
    }
}

// ---- the boards ----

struct Board {
    seed: u64,
    index: u64,
    kinds: Vec<SourceKind>,
    sources: Vec<ManaSource>,
    pool: ManaPool,
    cost: ManaCost,
    hand: Vec<ManaCost>,
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "board {} of seed {:#x}: sources [", self.index, self.seed)?;
        for (i, kind) in self.kinds.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "#{} {}", self.sources[i].object_id.0, kind_name(*kind))?;
        }
        write!(f, "], pool {}, cost {}, hand [", pool_str(&self.pool), self.cost)?;
        for (i, h) in self.hand.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{h}")?;
        }
        write!(f, "]")
    }
}

fn pool_str(pool: &ManaPool) -> String {
    let s: String = pool.mana.iter()
        .flat_map(|(&mt, &n)| std::iter::repeat_n(mana_letter(mt), n as usize))
        .map(|c| format!("{{{c}}}"))
        .collect();
    if s.is_empty() { "{}".to_string() } else { s }
}

fn mana_letter(mt: ManaType) -> char {
    match mt {
        ManaType::White => 'W',
        ManaType::Blue => 'U',
        ManaType::Black => 'B',
        ManaType::Red => 'R',
        ManaType::Green => 'G',
        ManaType::Colorless => 'C',
    }
}

const ALL_MANA: [ManaType; 6] = [
    ManaType::White, ManaType::Blue, ManaType::Black,
    ManaType::Red, ManaType::Green, ManaType::Colorless,
];

/// Which cost shapes a board's costs are drawn from.
#[derive(Clone, Copy)]
enum Shapes {
    /// The shapes cards in the pool actually have.
    Pool,
    /// Shapes no card in the pool has yet (see [`unmet_shape_cost`]).
    Unmet,
}

fn board(seed: u64, index: u64, shapes: Shapes) -> Board {
    let mut rng = board_rng(seed, index);
    let n_sources = 1 + rng.below(7);
    let mut kinds = Vec::new();
    let mut grottos = 0;
    for _ in 0..n_sources {
        let kind = match rng.below(12) {
            0..=4 => SourceKind::Basic(rng.pick(&Color::ALL)),
            5 | 6 => {
                let a = rng.pick(&Color::ALL);
                let mut b = rng.pick(&Color::ALL);
                while b == a { b = rng.pick(&Color::ALL); }
                SourceKind::Dual(a, b)
            }
            7 => SourceKind::SolRing,
            8 | 9 => SourceKind::Creature(rng.pick(&Color::ALL)),
            // Each Grotto multiplies the oracle's search by seven, and no
            // limited deck runs more than a couple; a third rolled becomes a
            // basic.
            10 if grottos < 2 => { grottos += 1; SourceKind::Grotto }
            10 => SourceKind::Basic(rng.pick(&Color::ALL)),
            _ => SourceKind::Assistant,
        };
        kinds.push(kind);
    }
    let sources: Vec<ManaSource> = kinds.iter().enumerate()
        .map(|(i, &kind)| source(i as u64 + 1, kind))
        .collect();
    let mut pool = ManaPool::new();
    for _ in 0..rng.below(4) {
        pool.add(rng.pick(&ALL_MANA), 1);
    }
    // Costs lean toward the colours the board makes, or most boards would
    // be able to pay nothing and the oracle would have nothing to compare.
    let mut board_colors: Vec<Color> = kinds.iter().flat_map(|&k| kind_colors(k)).collect();
    board_colors.sort_by_key(|c| letter(*c));
    board_colors.dedup();
    let mut color = |rng: &mut Rng| -> Color {
        if !board_colors.is_empty() && rng.below(4) != 0 {
            rng.pick(&board_colors)
        } else {
            rng.pick(&Color::ALL)
        }
    };
    let mut cost_of = |rng: &mut Rng| match shapes {
        Shapes::Pool => pool_shaped_cost(rng, &mut color),
        Shapes::Unmet => unmet_shape_cost(rng, &mut color),
    };
    let cost = cost_of(&mut rng);
    let hand = (0..rng.below(4)).map(|_| cost_of(&mut rng)).collect();
    Board { seed, index, kinds, sources, pool, cost, hand }
}

/// A cost shaped like one a card in the pool has: generic `{0}`–`{4}`, then
/// either no pips, one to three pips of one colour ({B}, {B}{B}, {3}{G}{G}{G}),
/// or one pip each of two colours ({1}{W}{B}, {2}{B}{R}). Nothing in the
/// pool costs three colours, a `{C}`, or two colours with one repeated.
fn pool_shaped_cost(rng: &mut Rng, color: &mut dyn FnMut(&mut Rng) -> Color) -> ManaCost {
    let mut symbols = Vec::new();
    let generic = rng.below(5) as u32;
    if generic > 0 {
        symbols.push(ManaSymbol::Generic(generic));
    }
    match rng.below(3) {
        0 => {}
        1 => {
            let c = color(rng);
            // One or two pips are common and three is rare, as in the pool.
            for _ in 0..rng.pick(&[1, 1, 1, 2, 2, 3]) {
                symbols.push(ManaSymbol::Colored(c));
            }
        }
        _ => {
            let a = color(rng);
            let mut b = color(rng);
            while b == a { b = color(rng); }
            symbols.push(ManaSymbol::Colored(a));
            symbols.push(ManaSymbol::Colored(b));
        }
    }
    if symbols.is_empty() {
        symbols.push(ManaSymbol::Generic(1));
    }
    ManaCost::new(symbols)
}

/// A cost shaped like none in the pool: three colours ({W}{U}{B}), a `{C}`
/// pip ({1}{C}, {C}{G}), or two colours with a repeated pip ({1}{W}{W}{B}).
/// The playtest of 2026-10-05 recorded these as shapes the greedy scarcity
/// sort misses plans for, and nothing asserts them: no card can meet them.
fn unmet_shape_cost(rng: &mut Rng, color: &mut dyn FnMut(&mut Rng) -> Color) -> ManaCost {
    let mut symbols = Vec::new();
    let generic = rng.below(3) as u32;
    if generic > 0 {
        symbols.push(ManaSymbol::Generic(generic));
    }
    let a = color(rng);
    let mut b = color(rng);
    while b == a { b = color(rng); }
    match rng.below(3) {
        0 => {
            let mut c = color(rng);
            while c == a || c == b { c = color(rng); }
            symbols.extend([ManaSymbol::Colored(a), ManaSymbol::Colored(b), ManaSymbol::Colored(c)]);
        }
        1 => {
            symbols.push(ManaSymbol::Colorless(1 + rng.below(2) as u32));
            if rng.below(2) == 0 {
                symbols.push(ManaSymbol::Colored(a));
            }
        }
        _ => {
            symbols.extend([ManaSymbol::Colored(a), ManaSymbol::Colored(a), ManaSymbol::Colored(b)]);
        }
    }
    ManaCost::new(symbols)
}

// ---- the oracle ----

fn ability_of<'a>(sources: &'a [&ManaSource], entry: (ObjectId, usize)) -> Option<&'a ManaAbilityDef> {
    sources.iter()
        .find(|s| s.object_id == entry.0)
        .and_then(|s| s.abilities.iter().find(|a| a.ability_index == entry.1))
}

fn is_free(a: &ManaAbilityDef) -> bool {
    a.cost.symbols.is_empty()
}

/// Visit every hand-tap plan over `sources` — every subset, one ability
/// each, free abilities ahead of cost-bearing ones as the engine orders a
/// plan — until `visit` returns true. Returns whether it did.
///
/// Free-first is the only order worth visiting: a filter funded by another
/// filter's output nets exactly what the second filter alone would, with
/// one more source tapped, so nothing is reachable through a chain that a
/// smaller free-first plan does not reach with more left untapped.
fn any_plan(sources: &[&ManaSource], visit: &mut dyn FnMut(&[(ObjectId, usize)]) -> bool) -> bool {
    fn go(
        sources: &[&ManaSource],
        i: usize,
        free: &mut Plan,
        paid: &mut Plan,
        visit: &mut dyn FnMut(&[(ObjectId, usize)]) -> bool,
    ) -> bool {
        if i == sources.len() {
            let plan: Plan = free.iter().chain(paid.iter()).copied().collect();
            return visit(&plan);
        }
        if go(sources, i + 1, free, paid, visit) {
            return true;
        }
        for a in &sources[i].abilities {
            let entry = (sources[i].object_id, a.ability_index);
            let stack = if is_free(a) { &mut *free } else { &mut *paid };
            stack.push(entry);
            let found = go(sources, i + 1, free, paid, visit);
            let stack = if is_free(a) { &mut *free } else { &mut *paid };
            stack.pop();
            if found {
                return true;
            }
        }
        false
    }
    go(sources, 0, &mut Vec::new(), &mut Vec::new(), visit)
}

/// Whether running `plan` can leave the pool able to pay `cost`, paying each
/// filter's `{1}` with whichever mana makes that true: what a person tapping
/// by hand can do, and so what the planner has to match.
fn plan_pays_somehow(plan: &[(ObjectId, usize)], pool: &ManaPool, sources: &[&ManaSource], cost: &ManaCost) -> bool {
    fn step(plan: &[(ObjectId, usize)], pool: ManaPool, sources: &[&ManaSource], cost: &ManaCost) -> bool {
        let Some((&entry, rest)) = plan.split_first() else {
            return mana::can_pay(&pool, cost);
        };
        let a = ability_of(sources, entry).expect("the oracle names abilities the sources have");
        assert!(a.cost.colored_requirements().is_empty() && a.cost.colorless_amount() == 0,
            "the oracle pays generic ability costs only, and the pool's filters cost {{1}}");
        after_paying_generic(pool, a.cost.generic_amount(), &mut |mut paid| {
            for &(mt, n) in &a.produced {
                paid.add(mt, n);
            }
            step(rest, paid, sources, cost)
        })
    }
    step(plan, pool.clone(), sources, cost)
}

/// Every pool reachable by spending `n` mana of any types out of `pool`,
/// until `then` returns true.
fn after_paying_generic(pool: ManaPool, n: u32, then: &mut dyn FnMut(ManaPool) -> bool) -> bool {
    if n == 0 {
        return then(pool);
    }
    for mt in ALL_MANA {
        if pool.get(mt) > 0 {
            let mut p = pool.clone();
            p.sub(mt, 1);
            if after_paying_generic(p, n - 1, then) {
                return true;
            }
        }
    }
    false
}

/// Run `plan` and pay `cost` the way the engine does — each filter's cost
/// paid around what `cost` needs, then `cost` paid around what the rest of
/// the hand needs (`mana::auto_pay_reserving`, issues #252 and #678) — and
/// return what is left, or `None` if the plan does not pay that way.
fn pool_after(
    plan: &[(ObjectId, usize)],
    pool: &ManaPool,
    sources: &[&ManaSource],
    cost: &ManaCost,
    reserve: &ManaCost,
) -> Option<ManaPool> {
    let mut pool = pool.clone();
    for &entry in plan {
        let a = ability_of(sources, entry)?;
        mana::auto_pay_reserving(&mut pool, &a.cost, cost).ok()?;
        for &(mt, n) in &a.produced {
            pool.add(mt, n);
        }
    }
    mana::auto_pay_reserving(&mut pool, cost, reserve).ok()?;
    Some(pool)
}

/// The oracle for one board, with its castability answers memoised: the
/// stranding count asks "can the untapped sources plus what floats pay
/// this hand spell" once per plan per hand spell, and most plans leave the
/// same few boards behind.
struct Oracle<'a> {
    board: &'a Board,
    /// The hand spells some hand-tap pays on the whole board, before `cost`.
    castable_before: Vec<&'a ManaCost>,
    reserve: ManaCost,
    memo: HashMap<(Vec<ObjectId>, Vec<(ManaType, u32)>, usize), bool>,
}

impl<'a> Oracle<'a> {
    fn new(board: &'a Board) -> Self {
        let all: Vec<&ManaSource> = board.sources.iter().collect();
        let castable_before = board.hand.iter()
            .filter(|h| Self::pays(&all, &board.pool, h))
            .collect();
        Self { board, castable_before, reserve: mana::hand_reserve(&board.hand), memo: HashMap::new() }
    }

    /// Whether any hand-tap over `sources` with `pool` floating pays `cost`.
    fn pays(sources: &[&ManaSource], pool: &ManaPool, cost: &ManaCost) -> bool {
        any_plan(sources, &mut |plan| plan_pays_somehow(plan, pool, sources, cost))
    }

    fn untapped(&self, plan: &[(ObjectId, usize)]) -> Vec<&'a ManaSource> {
        self.board.sources.iter()
            .filter(|s| !plan.iter().any(|&(id, _)| id == s.object_id))
            .collect()
    }

    /// How many hand spells castable before `plan` are not castable from
    /// what it leaves — or `None` if the plan does not pay the engine's way.
    fn stranded(&mut self, plan: &[(ObjectId, usize)]) -> Option<usize> {
        let all: Vec<&ManaSource> = self.board.sources.iter().collect();
        let left = pool_after(plan, &self.board.pool, &all, &self.board.cost, &self.reserve)?;
        let untapped = self.untapped(plan);
        let ids: Vec<ObjectId> = untapped.iter().map(|s| s.object_id).collect();
        let pool_key: Vec<(ManaType, u32)> = left.mana.iter()
            .filter(|&(_, &n)| n > 0)
            .map(|(&mt, &n)| (mt, n))
            .collect();
        let mut count = 0;
        for (i, h) in self.castable_before.iter().enumerate() {
            let key = (ids.clone(), pool_key.clone(), i);
            let payable = match self.memo.get(&key) {
                Some(&p) => p,
                None => {
                    let p = Self::pays(&untapped, &left, h);
                    self.memo.insert(key, p);
                    p
                }
            };
            if !payable {
                count += 1;
            }
        }
        Some(count)
    }

    /// How many of the sources `plan` taps have side effects.
    fn side_effects(&self, plan: &[(ObjectId, usize)]) -> usize {
        self.board.sources.iter()
            .filter(|s| s.source_kind == ManaSourceKind::HasSideEffects)
            .filter(|s| plan.iter().any(|&(id, _)| id == s.object_id))
            .count()
    }

    /// The fewest hand spells any plan for `cost` that taps at most
    /// `max_side_effects` side-effect sources strands, with a plan that
    /// strands that few.
    fn least_stranding_plan(&mut self, max_side_effects: usize) -> Option<(usize, Plan)> {
        let all: Vec<&ManaSource> = self.board.sources.iter().collect();
        let mut best: Option<(usize, Plan)> = None;
        any_plan(&all, &mut |plan| {
            if self.side_effects(plan) > max_side_effects {
                return false;
            }
            if let Some(n) = self.stranded(plan) {
                if best.as_ref().is_none_or(|(b, _)| n < *b) {
                    best = Some((n, plan.to_vec()));
                }
            }
            best.as_ref().is_some_and(|(b, _)| *b == 0)
        });
        best
    }
}

// ---- the properties ----

/// Which of the planner's promises a sweep holds it to. Soundness — a plan
/// it offers executes, taps nothing for nothing and names only sources it
/// was given — is always checked.
#[derive(Clone, Copy)]
struct Checks {
    /// 1. A plan is offered whenever some hand-tap pays.
    completeness: bool,
    /// 3. The plan strands no more of the hand than any hand-tap that taps
    ///    no more side-effect sources would.
    stranding: bool,
}

/// Every way `board` shows the planner breaking its contract, as messages.
/// Empty when it holds.
fn violations(board: &Board, checks: Checks) -> Vec<String> {
    let mut found = Vec::new();
    let all: Vec<&ManaSource> = board.sources.iter().collect();
    let mut oracle = Oracle::new(board);
    let plan = mana::compute_autotap(&board.cost, &board.pool, &board.sources, &board.hand);

    // 1. No missing plan.
    let Some(plan) = plan else {
        if checks.completeness && Oracle::pays(&all, &board.pool, &board.cost) {
            let mut witness = None;
            any_plan(&all, &mut |p| {
                let pays = plan_pays_somehow(p, &board.pool, &all, &board.cost);
                if pays { witness = Some(p.to_vec()); }
                pays
            });
            found.push(format!("{board}: no plan offered for {}, but {:?} pays it",
                board.cost, witness.expect("the oracle just found one")));
        }
        return found;
    };

    // 5. Only sources it was given, each once.
    let mut seen = Vec::new();
    for &entry in &plan {
        if ability_of(&all, entry).is_none() {
            found.push(format!("{board}: {plan:?} names {entry:?}, which is not a source and ability on the board"));
        }
        if seen.contains(&entry.0) {
            found.push(format!("{board}: {plan:?} taps {} twice", entry.0));
        }
        seen.push(entry.0);
    }
    if !found.is_empty() {
        return found;
    }

    // 2. The plan executes, the way the engine executes it.
    let Some(stranded) = oracle.stranded(&plan) else {
        found.push(format!("{board}: the plan {plan:?} offered for {} does not pay it", board.cost));
        return found;
    };

    // 3. No stranding when avoidable. Avoidable without a side effect, that
    // is: the planner never taps a Deranged Assistant to keep a colour (its
    // doc comment: milling a card to save a colour is not a win), so the
    // hand-taps it is held to are the ones tapping no more such sources.
    if checks.stranding {
        let (least, by) = oracle.least_stranding_plan(oracle.side_effects(&plan))
            .expect("the planner's own plan pays");
        if stranded > least {
            let castable: Vec<String> = oracle.castable_before.iter().map(|h| h.to_string()).collect();
            found.push(format!(
                "{board}: the plan {plan:?} strands {stranded} of [{}], but {by:?} strands {least}",
                castable.join(", ")));
        }
    }

    // 4. No pointless tap: dropping any one entry stops the plan paying or
    // strands strictly more.
    for i in 0..plan.len() {
        let mut shorter = plan.clone();
        let dropped = shorter.remove(i);
        if let Some(n) = oracle.stranded(&shorter) {
            if n <= stranded {
                found.push(format!(
                    "{board}: {plan:?} taps {dropped:?} for nothing — without it the plan still pays {} \
                     and strands {n} (not more than {stranded})", board.cost));
            }
        }
    }
    found
}

fn sweep(shapes: Shapes, checks: Checks) -> Vec<String> {
    let seed = std::env::var("AUTOTAP_BRUTE_FORCE_SEED").ok()
        .map(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).expect("a hex seed"))
        .unwrap_or(DEFAULT_SEED);
    let boards = std::env::var("AUTOTAP_BRUTE_FORCE_BOARDS").ok()
        .map(|s| s.parse().expect("a board count"))
        .unwrap_or(DEFAULT_BOARDS);
    let mut found = Vec::new();
    for index in 0..boards {
        found.extend(violations(&board(seed, index, shapes), checks));
    }
    found
}

fn report(found: &[String], what: &str) {
    assert!(found.is_empty(),
        "{} board(s) where {what}; the first {}:\n  {}",
        found.len(), found.len().min(10), found[..found.len().min(10)].join("\n  "));
}

/// Over seeded random boards with the cost shapes the card pool has, the
/// planner offers a plan whenever tapping by hand would pay, and offers
/// only plans the engine can execute, naming sources it was given once each
/// and tapping nothing it did not need.
#[test]
fn the_planner_matches_tapping_by_hand_on_random_boards() {
    report(&sweep(Shapes::Pool, Checks { completeness: true, stranding: false }),
        "the planner broke its contract");
}

/// And its plan strands no more of the hand than the best hand-tap that
/// taps no more side-effect sources would.
///
/// Ignored until issue #683 is fixed, because the planner does not promise
/// this much yet. What it does promise (`keep_the_hand_castable`'s doc
/// comment) is the best plan within one swapped source or one extra source
/// of its greedy plan — and the greedy plan is private to `mana.rs`, so
/// that promise cannot be stated here in its own terms. The global property
/// fails on 2 of the default sweep's 2,000 boards (228 and 493), both
/// repairs two moves from the greedy plan; #683 carries the shrunk repro.
/// When the planner keeps the global promise, drop the `#[ignore]`. Board 493 shrinks to: Forest, Pilgrim(B), Mountain and
/// a W/B dual, `{W}{U}` floating, casting `{2}{B}{R}` with `{W}{W}` in
/// hand. The planner taps Mountain and the dual for `{B}` and pays the
/// generic from the pool, so no White is left anywhere; tapping Forest,
/// Pilgrim and Mountain instead leaves the `{W}` floating and the dual
/// untapped for the second. That is a swap (dual for Pilgrim) and an extra
/// source (Forest) at once. Run with `-- --ignored` for the current list.
#[test]
#[ignore = "issue #683: the planner promises a one-move repair, not the global optimum; see the doc comment"]
fn a_plan_strands_no_more_of_the_hand_than_any_hand_tap_would() {
    report(&sweep(Shapes::Pool, Checks { completeness: false, stranding: true }),
        "a hand-tap would have stranded less");
}

/// The same sweep with costs no card in the pool has yet — three colours,
/// `{C}` pips, two colours with a repeated pip. Only soundness is asserted,
/// because the planner is known to fall short of the rest on these and
/// nothing in the pool can meet them yet (the 2026-10-05 playtest recorded
/// them as an idea rather than a bug; the `{C}` misses are issue #684 and
/// the stranding is #683). Set either check to true to see the current
/// list; on the default sweep it is:
/// - no plan for a `{C}` pip when the free `{C}` is also the only route to
///   the colour: Grotto and Assistant, `{G}` floating, casting `{C}{R}`.
///   Phase 1 spends the Grotto's free `{C}` on the pip (tier 1 beats the
///   Assistant's tier 4) and nothing is left to make the `{R}`, where the
///   Assistant's `{C}` and the Grotto filtering `{G}` into `{R}` would pay.
///   Four boards, all `{C}`; three-colour and repeated-pip costs missed no
///   plan on any board.
/// - stranding a three-colour or repeated-pip hand spell that two more
///   sources would have kept castable: Pilgrim(B), Mountain, Mountain and
///   an R/G dual, `{U}{U}{G}` floating, casting `{1}{U}{R}{G}` with
///   `{B}{R}{U}` in hand. One Mountain pays, and the pool's `{U}` goes on
///   the generic; tapping both Mountains and the dual leaves `{U}{R}`
///   floating for the Pilgrim's `{B}`. Five boards.
#[test]
fn a_plan_for_an_unmet_cost_shape_still_executes() {
    report(&sweep(Shapes::Unmet, Checks { completeness: false, stranding: false }),
        "the planner broke its contract on an unmet shape");
}

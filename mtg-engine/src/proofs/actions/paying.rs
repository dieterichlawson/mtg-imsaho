//! `engine::pay_cost_with_sources` and `payment_plan_with_sources`, which a
//! card proof's `record_pay` and `any_payment_plan` stand in for: a cost a
//! resolving effect lets a player pay ("you may pay {1}"), from floating
//! mana or by tapping what they control (CR 608.2g, 605.3a).
//!
//! Which mana a payment spends is `mana::auto_pay_reserving`'s, proved in
//! `mtg-kernel` for every pool, cost and reserve, and whether a pool can
//! pay a cost is `mana::can_pay`'s, proved there too: what is proved here
//! is what the engine does around them. The boards print nothing a player
//! could pay for besides (no hand, no ability with a cost), so nothing is
//! held in reserve.

use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::proofs::cards::{blank_board, board, Snapshot};
use crate::types::{Color, ManaCost, ManaPool, ManaSymbol, ManaType, Zone};

/// Any pool of up to two white, two green and two colorless mana; but, if
/// `held` is given, exactly that much of the cost's colour (`colour_of`).
fn any_pool(colour: Option<ManaType>, held: Option<u32>) -> ManaPool {
    let mut pool = ManaPool::new();
    for t in [ManaType::White, ManaType::Green, ManaType::Colorless] {
        let n = match (colour, held) {
            (Some(c), Some(h)) if c == t => h,
            _ => {
                let n: u8 = kani::any();
                kani::assume(n <= 2);
                u32::from(n)
            }
        };
        if n > 0 {
            pool.add(t, n);
        }
    }
    pool
}

/// The colour cost `which` (`a_cost`) has pips of, if any.
fn colour_of(which: u8) -> Option<ManaType> {
    match which {
        1 | 2 | 5 => Some(ManaType::Green),
        3 => Some(ManaType::White),
        _ => None,
    }
}

/// The costs the proofs pay, one per call: `{1}`, `{G}`, `{1}{G}`,
/// `{W}{W}`, `{C}` and `{G}{G}`.
fn a_cost(which: u8) -> ManaCost {
    ManaCost::new(match which {
        0 => vec![ManaSymbol::Generic(1)],
        1 => vec![ManaSymbol::Colored(Color::Green)],
        2 => vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Green)],
        3 => vec![ManaSymbol::Colored(Color::White), ManaSymbol::Colored(Color::White)],
        4 => vec![ManaSymbol::Colorless(1)],
        _ => vec![ManaSymbol::Colored(Color::Green), ManaSymbol::Colored(Color::Green)],
    })
}

/// What `pool` holds once `cost` is paid from it, nothing held in reserve.
fn spent(pool: &ManaPool, cost: &ManaCost) -> ManaPool {
    let mut left = pool.clone();
    assert!(mana::auto_pay_reserving(&mut left, cost, &ManaCost::free()).is_ok());
    left
}

/// With nothing to tap, a cost is paid exactly when the floating mana pays
/// it (CR 118.3): the plan is to tap nothing, the pool spends what paying
/// spends, and the answer is that it was paid. One the pool cannot pay is
/// refused, with no plan, and nothing changes. For each of six costs and
/// any pool of up to two of three types, either player's.
///
/// `who` pays cost `which` with nothing to tap: a permanent of each
/// player's that makes no mana; the pool holding `held` of the cost's
/// colour, if given, a constant in each harness. With that the solver's
/// too, the pips still owed after the pool paid what it could were a list
/// of a length it left open, which the planner sorts and walks, and the
/// proofs of every cost with a colour ran out of memory in the solver at
/// 7 GB. A harness may then reach one outcome only (no green never pays
/// {G}), and its other cover is unreached.
fn from_the_pool(which: u8, who: PlayerId, held: Option<u32>) {
    let (registry, mut state, blank) = blank_board();
    state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    state.create_object(blank, PlayerId(1), Zone::Battlefield, None, None);
    let cost = a_cost(which);
    let pool = any_pool(colour_of(which), held);
    state.players[usize::from(who.0)].mana_pool = pool.clone();
    let before = Snapshot::of(&state);
    let can = mana::can_pay(&pool, &cost);

    let plan = crate::engine::payment_plan_with_sources(&state, who, &cost, &registry);
    assert!(plan == can.then(Vec::new));
    let paid = crate::engine::pay_cost_with_sources(&mut state, who, &cost, &registry);

    assert!(paid == can);
    let expected = if can { spent(&pool, &cost) } else { pool.clone() };
    assert!(state.players[usize::from(who.0)].mana_pool == expected);
    assert!(state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    assert!(Snapshot::of(&state) == before);
    // Both outcomes reached, for every cost a pool of any colour pays;
    // with the colour's amount given, the one that amount allows.
    kani::cover!(paid, "the cost paid from the pool");
    kani::cover!(!paid, "the cost refused");
    std::mem::forget((registry, state, cost, pool, expected, plan));
}


/// A Forest's board: `who` pays cost `which` with a Forest of theirs,
/// untapped if `untapped`, its summoning sickness any, and a permanent of
/// the other player's that makes no mana; the pool any of up to two of
/// three types, holding `held` of the cost's colour if given. What the
/// payment should come to: whether the pool pays alone, and whether the
/// Forest is to be tapped for the rest.
struct ForestBoard {
    registry: crate::cards::CardRegistry,
    state: crate::state::GameState,
    forest: ObjectId,
    cost: ManaCost,
    pool: ManaPool,
    with_forest: ManaPool,
    alone: bool,
    tap: bool,
    before: Snapshot,
}

fn forest_board(which: u8, who: PlayerId, held: Option<u32>, untapped: bool) -> ForestBoard {
    let (registry, mut state, blank, forest_card) = board(Box::new(crate::cards::forest::Forest));
    let forest = state.create_object(forest_card, who, Zone::Battlefield, None, None);
    let o = state.get_object_mut(forest).expect("just created");
    o.tapped = !untapped;
    o.summoning_sick = kani::any();
    state.create_object(blank, PlayerId(1 - who.0), Zone::Battlefield, None, None);
    let cost = a_cost(which);
    let pool = any_pool(colour_of(which), held);
    state.players[usize::from(who.0)].mana_pool = pool.clone();
    let before = Snapshot::of(&state);
    // The pool alone, or with the Forest's {G} (CR 605.3a: a mana ability
    // may be activated while paying), and the Forest only when the pool
    // alone falls short.
    let mut with_forest = pool.clone();
    with_forest.add(ManaType::Green, 1);
    let alone = mana::can_pay(&pool, &cost);
    let tap = !alone && untapped && mana::can_pay(&with_forest, &cost);
    ForestBoard { registry, state, forest, cost, pool, with_forest, alone, tap, before }
}

/// A cost the floating mana does not pay is paid by tapping an untapped
/// land of the player's, a land's mana not waiting on summoning sickness
/// (CR 302.6 is of creatures): the plan names the Forest exactly when the
/// pool falls short and its {G} makes up the difference; a cost the pool
/// pays alone plans to tap nothing, and one neither pays has no plan. For
/// each of the six costs, either player, the Forest untapped or not, and
/// any pool of up to two of three types, holding a constant amount of the
/// cost's colour (`from_the_pool` says why), each a constant in each
/// harness. Paying is the next proof's: `pay_cost_with_sources` plans
/// before it taps, and a harness that planned and then paid planned
/// twice.
fn planning_with_a_forest(which: u8, who: PlayerId, held: Option<u32>, untapped: bool) {
    let f = forest_board(which, who, held, untapped);
    let plan = crate::engine::payment_plan_with_sources(&f.state, who, &f.cost, &f.registry);
    let expected: Option<Vec<(ObjectId, usize)>> = if f.alone {
        Some(Vec::new())
    } else if f.tap {
        Some(vec![(f.forest, 0)])
    } else {
        None
    };
    assert!(plan == expected);
    kani::cover!(f.alone, "the pool alone");
    kani::cover!(f.tap, "the pool and the Forest");
    kani::cover!(!f.alone && !f.tap, "neither");
    std::mem::forget((f, plan, expected));
}

/// Any pool of up to two white, two green and two colorless mana, but
/// with exactly the amounts `held` gives of the types it names.
fn pool_holding(held: &[(ManaType, u32)]) -> ManaPool {
    let mut pool = ManaPool::new();
    for t in [ManaType::White, ManaType::Green, ManaType::Colorless] {
        let mut n = None;
        for &(h, amount) in held {
            if h == t {
                n = Some(amount);
            }
        }
        let n = n.unwrap_or_else(|| {
            let n: u8 = kani::any();
            kani::assume(n <= 2);
            u32::from(n)
        });
        if n > 0 {
            pool.add(t, n);
        }
    }
    pool
}

/// [`paying_with_a_forest`] by an untapped Forest, the pool holding the
/// amounts `held` gives (`pool_holding`): each harness one pool, and
/// together the pools `paying_with_a_forest` is over. Over a pool the
/// solver chose, paying `{1}`, and `{1}{G}` with green floating, ran out
/// of memory at 12 GB (41 million clauses), `{1}` with its green held as
/// well: a generic pip is paid by ordering what the pool holds beyond the
/// colours' needs, and over amounts left open that order was the
/// formula. Its board is `forest_board`'s with those amounts given, built
/// beside it rather than through it so that the proofs over that board
/// are of the code they were run on.
fn paying_with_a_forest_holding(which: u8, who: PlayerId, held: &[(ManaType, u32)]) {
    let (registry, mut state, blank, forest_card) = board(Box::new(crate::cards::forest::Forest));
    let forest = state.create_object(forest_card, who, Zone::Battlefield, None, None);
    state.get_object_mut(forest).expect("just created").summoning_sick = kani::any();
    state.create_object(blank, PlayerId(1 - who.0), Zone::Battlefield, None, None);
    let cost = a_cost(which);
    let pool = pool_holding(held);
    state.players[usize::from(who.0)].mana_pool = pool.clone();
    let before = Snapshot::of(&state);
    let mut with_forest = pool.clone();
    with_forest.add(ManaType::Green, 1);
    let alone = mana::can_pay(&pool, &cost);
    let tap = !alone && mana::can_pay(&with_forest, &cost);
    let paid = crate::engine::pay_cost_with_sources(&mut state, who, &cost, &registry);
    assert!(paid == (alone || tap));
    let expected = if alone {
        spent(&pool, &cost)
    } else if tap {
        spent(&with_forest, &cost)
    } else {
        pool.clone()
    };
    assert!(state.players[usize::from(who.0)].mana_pool == expected);
    assert!(state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    if tap {
        assert!(matches!(state.events.as_slice(), [GameEvent::Tapped { object }, GameEvent::ManaAdded { player, mana_type: ManaType::Green, amount: 1 }]
            if *object == forest && *player == who));
        assert!(Snapshot::of(&state).without_events() == before.with_tapped(forest, true).without_events());
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    std::mem::forget((registry, state, cost, pool, with_forest, expected));
}

/// Activating the Forest's ability, as a payment by tapping it does (CR
/// 605.3a): the Forest tapped, then its {G} added to its controller's
/// pool, announced in that order, and nothing else; over any pool, the
/// Forest untapped, its summoning sickness any (a land's mana does not
/// wait on it, CR 302.6). This is what [`forest_activation`] does, which
/// stands in for the activation in [`paying_with_a_forest`]: the
/// payment's proofs follow the planner, the payment from the pool and
/// the board, and this one the activation, once.
fn activating_the_forest(who: PlayerId) {
    let mut f = forest_board(1, who, None, true);
    crate::engine::activate_mana_source_reserving(&mut f.state, f.forest, 0, Some(&f.cost), &f.registry);
    assert!(f.state.players[usize::from(who.0)].mana_pool == f.with_forest);
    assert!(f.state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    assert!(matches!(f.state.events.as_slice(), [GameEvent::Tapped { object }, GameEvent::ManaAdded { player, mana_type: ManaType::Green, amount: 1 }]
        if *object == f.forest && *player == who));
    assert!(Snapshot::of(&f.state).without_events() == f.before.with_tapped(f.forest, true).without_events());
    std::mem::forget(f);
}

/// Stands in for `engine::activate_mana_source_reserving` in the proofs of
/// paying with a Forest: on their boards the only source a plan can name
/// is the Forest's ability, and [`activating_the_forest`] proves that the
/// engine's activation of it is this.
#[allow(dead_code)] // named only by `kani::stub`
fn forest_activation(
    state: &mut crate::state::GameState,
    source: ObjectId,
    _: usize,
    _: Option<&ManaCost>,
    _: &crate::cards::CardRegistry,
) {
    let controller = state.get_object(source).expect("the Forest").controller;
    state.tap(source);
    state.add_mana(controller, ManaType::Green, 1);
}

/// Paying as [`planning_with_a_forest`] plans: the Forest, when it is to be
/// tapped, tapped and its {G} added, announced in that order, and the cost
/// paid from the pool so made; a cost the pool pays alone paid from it,
/// tapping nothing; one neither pays refused, nothing changed. The
/// Forest's activation is [`forest_activation`]'s here, which
/// [`activating_the_forest`] proves the engine's is: with the engine's own
/// followed, the proof of tapping it for {G} ran out of memory at 7 GB in
/// symbolic execution.
fn paying_with_a_forest(which: u8, who: PlayerId, held: Option<u32>, untapped: bool) {
    let mut f = forest_board(which, who, held, untapped);
    let paid = crate::engine::pay_cost_with_sources(&mut f.state, who, &f.cost, &f.registry);
    assert!(paid == (f.alone || f.tap));
    let expected = if f.alone {
        spent(&f.pool, &f.cost)
    } else if f.tap {
        spent(&f.with_forest, &f.cost)
    } else {
        f.pool.clone()
    };
    assert!(f.state.players[usize::from(who.0)].mana_pool == expected);
    assert!(f.state.players[usize::from(1 - who.0)].mana_pool.is_empty());
    if f.tap {
        assert!(matches!(f.state.events.as_slice(), [GameEvent::Tapped { object }, GameEvent::ManaAdded { player, mana_type: ManaType::Green, amount: 1 }]
            if *object == f.forest && *player == who));
        assert!(Snapshot::of(&f.state).without_events() == f.before.with_tapped(f.forest, true).without_events());
    } else {
        assert!(Snapshot::of(&f.state) == f.before);
    }
    kani::cover!(f.alone, "the cost paid from the pool alone");
    kani::cover!(f.tap, "the cost paid with the Forest's green");
    kani::cover!(!f.alone && !f.tap, "the cost refused");
    std::mem::forget((f, expected));
}

/// A proof above for each cost and each player paying, both constants in
/// each harness, and how much of the cost's colour the pool holds, if
/// given: a player index left to the solver blurs both players' pools
/// into one question, a cost left to it is a list of every cost at once,
/// and the cases of one harness add up.
macro_rules! each_cost {
    ($($name:ident: $proof:ident($which:literal, $who:literal, $held:expr);)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        /// The proof above it, for one cost and one player.
        fn $name() {
            $proof($which, PlayerId($who), $held);
        }
    )*};
}

each_cost! {
    one_is_paid_from_the_pool_exactly_when_it_can_be_for_the_first_player: from_the_pool(0, 0, None);
    one_is_paid_from_the_pool_exactly_when_it_can_be_for_the_second_player: from_the_pool(0, 1, None);
    green_is_paid_from_the_pool_exactly_when_it_can_be_with_no_green_for_the_first_player: from_the_pool(1, 0, Some(0));
    green_is_paid_from_the_pool_exactly_when_it_can_be_with_no_green_for_the_second_player: from_the_pool(1, 1, Some(0));
    green_is_paid_from_the_pool_exactly_when_it_can_be_with_one_green_for_the_first_player: from_the_pool(1, 0, Some(1));
    green_is_paid_from_the_pool_exactly_when_it_can_be_with_one_green_for_the_second_player: from_the_pool(1, 1, Some(1));
    green_is_paid_from_the_pool_exactly_when_it_can_be_with_two_green_for_the_first_player: from_the_pool(1, 0, Some(2));
    green_is_paid_from_the_pool_exactly_when_it_can_be_with_two_green_for_the_second_player: from_the_pool(1, 1, Some(2));
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_with_no_green_for_the_first_player: from_the_pool(2, 0, Some(0));
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_with_no_green_for_the_second_player: from_the_pool(2, 1, Some(0));
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_with_one_green_for_the_first_player: from_the_pool(2, 0, Some(1));
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_with_one_green_for_the_second_player: from_the_pool(2, 1, Some(1));
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_with_two_green_for_the_first_player: from_the_pool(2, 0, Some(2));
    one_and_green_are_paid_from_the_pool_exactly_when_they_can_be_with_two_green_for_the_second_player: from_the_pool(2, 1, Some(2));
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_with_no_white_for_the_first_player: from_the_pool(3, 0, Some(0));
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_with_no_white_for_the_second_player: from_the_pool(3, 1, Some(0));
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_with_one_white_for_the_first_player: from_the_pool(3, 0, Some(1));
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_with_one_white_for_the_second_player: from_the_pool(3, 1, Some(1));
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_with_two_white_for_the_first_player: from_the_pool(3, 0, Some(2));
    two_white_are_paid_from_the_pool_exactly_when_they_can_be_with_two_white_for_the_second_player: from_the_pool(3, 1, Some(2));
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_with_no_green_for_the_first_player: from_the_pool(5, 0, Some(0));
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_with_no_green_for_the_second_player: from_the_pool(5, 1, Some(0));
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_with_one_green_for_the_first_player: from_the_pool(5, 0, Some(1));
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_with_one_green_for_the_second_player: from_the_pool(5, 1, Some(1));
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_with_two_green_for_the_first_player: from_the_pool(5, 0, Some(2));
    two_green_are_paid_from_the_pool_exactly_when_they_can_be_with_two_green_for_the_second_player: from_the_pool(5, 1, Some(2));
    colorless_is_paid_from_the_pool_exactly_when_it_can_be_for_the_first_player: from_the_pool(4, 0, None);
    colorless_is_paid_from_the_pool_exactly_when_it_can_be_for_the_second_player: from_the_pool(4, 1, None);
}

/// A Forest proof above for each cost, each player paying, the Forest
/// untapped or not, and how much of the cost's colour the pool holds, if
/// it has one: each a constant in each harness, as [`each_cost`] says.
macro_rules! each_forest {
    ($($name:ident: $proof:ident($which:literal, $who:literal, $held:expr, $untapped:literal);)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::engine::activate_mana_source_reserving, forest_activation)]
        /// The proof above it, for one cost, one player and one Forest.
        fn $name() {
            $proof($which, PlayerId($who), $held, $untapped);
        }
    )*};
}

each_forest! {
    one_is_planned_with_an_untapped_forest_for_the_first_player: planning_with_a_forest(0, 0, None, true);
    one_is_planned_with_an_untapped_forest_for_the_second_player: planning_with_a_forest(0, 1, None, true);
    one_is_planned_with_a_tapped_forest_for_the_first_player: planning_with_a_forest(0, 0, None, false);
    one_is_planned_with_a_tapped_forest_for_the_second_player: planning_with_a_forest(0, 1, None, false);
    green_is_planned_with_an_untapped_forest_and_no_green_floating_for_the_first_player: planning_with_a_forest(1, 0, Some(0), true);
    green_is_planned_with_an_untapped_forest_and_no_green_floating_for_the_second_player: planning_with_a_forest(1, 1, Some(0), true);
    green_is_planned_with_an_untapped_forest_and_one_green_floating_for_the_first_player: planning_with_a_forest(1, 0, Some(1), true);
    green_is_planned_with_an_untapped_forest_and_one_green_floating_for_the_second_player: planning_with_a_forest(1, 1, Some(1), true);
    green_is_planned_with_an_untapped_forest_and_two_green_floating_for_the_first_player: planning_with_a_forest(1, 0, Some(2), true);
    green_is_planned_with_an_untapped_forest_and_two_green_floating_for_the_second_player: planning_with_a_forest(1, 1, Some(2), true);
    green_is_planned_with_a_tapped_forest_and_no_green_floating_for_the_first_player: planning_with_a_forest(1, 0, Some(0), false);
    green_is_planned_with_a_tapped_forest_and_no_green_floating_for_the_second_player: planning_with_a_forest(1, 1, Some(0), false);
    green_is_planned_with_a_tapped_forest_and_one_green_floating_for_the_first_player: planning_with_a_forest(1, 0, Some(1), false);
    green_is_planned_with_a_tapped_forest_and_one_green_floating_for_the_second_player: planning_with_a_forest(1, 1, Some(1), false);
    green_is_planned_with_a_tapped_forest_and_two_green_floating_for_the_first_player: planning_with_a_forest(1, 0, Some(2), false);
    green_is_planned_with_a_tapped_forest_and_two_green_floating_for_the_second_player: planning_with_a_forest(1, 1, Some(2), false);
    one_and_green_are_planned_with_an_untapped_forest_and_no_green_floating_for_the_first_player: planning_with_a_forest(2, 0, Some(0), true);
    one_and_green_are_planned_with_an_untapped_forest_and_no_green_floating_for_the_second_player: planning_with_a_forest(2, 1, Some(0), true);
    one_and_green_are_planned_with_an_untapped_forest_and_one_green_floating_for_the_first_player: planning_with_a_forest(2, 0, Some(1), true);
    one_and_green_are_planned_with_an_untapped_forest_and_one_green_floating_for_the_second_player: planning_with_a_forest(2, 1, Some(1), true);
    one_and_green_are_planned_with_an_untapped_forest_and_two_green_floating_for_the_first_player: planning_with_a_forest(2, 0, Some(2), true);
    one_and_green_are_planned_with_an_untapped_forest_and_two_green_floating_for_the_second_player: planning_with_a_forest(2, 1, Some(2), true);
    one_and_green_are_planned_with_a_tapped_forest_and_no_green_floating_for_the_first_player: planning_with_a_forest(2, 0, Some(0), false);
    one_and_green_are_planned_with_a_tapped_forest_and_no_green_floating_for_the_second_player: planning_with_a_forest(2, 1, Some(0), false);
    one_and_green_are_planned_with_a_tapped_forest_and_one_green_floating_for_the_first_player: planning_with_a_forest(2, 0, Some(1), false);
    one_and_green_are_planned_with_a_tapped_forest_and_one_green_floating_for_the_second_player: planning_with_a_forest(2, 1, Some(1), false);
    one_and_green_are_planned_with_a_tapped_forest_and_two_green_floating_for_the_first_player: planning_with_a_forest(2, 0, Some(2), false);
    one_and_green_are_planned_with_a_tapped_forest_and_two_green_floating_for_the_second_player: planning_with_a_forest(2, 1, Some(2), false);
    two_white_are_planned_with_an_untapped_forest_and_no_white_floating_for_the_first_player: planning_with_a_forest(3, 0, Some(0), true);
    two_white_are_planned_with_an_untapped_forest_and_no_white_floating_for_the_second_player: planning_with_a_forest(3, 1, Some(0), true);
    two_white_are_planned_with_an_untapped_forest_and_one_white_floating_for_the_first_player: planning_with_a_forest(3, 0, Some(1), true);
    two_white_are_planned_with_an_untapped_forest_and_one_white_floating_for_the_second_player: planning_with_a_forest(3, 1, Some(1), true);
    two_white_are_planned_with_an_untapped_forest_and_two_white_floating_for_the_first_player: planning_with_a_forest(3, 0, Some(2), true);
    two_white_are_planned_with_an_untapped_forest_and_two_white_floating_for_the_second_player: planning_with_a_forest(3, 1, Some(2), true);
    two_white_are_planned_with_a_tapped_forest_and_no_white_floating_for_the_first_player: planning_with_a_forest(3, 0, Some(0), false);
    two_white_are_planned_with_a_tapped_forest_and_no_white_floating_for_the_second_player: planning_with_a_forest(3, 1, Some(0), false);
    two_white_are_planned_with_a_tapped_forest_and_one_white_floating_for_the_first_player: planning_with_a_forest(3, 0, Some(1), false);
    two_white_are_planned_with_a_tapped_forest_and_one_white_floating_for_the_second_player: planning_with_a_forest(3, 1, Some(1), false);
    two_white_are_planned_with_a_tapped_forest_and_two_white_floating_for_the_first_player: planning_with_a_forest(3, 0, Some(2), false);
    two_white_are_planned_with_a_tapped_forest_and_two_white_floating_for_the_second_player: planning_with_a_forest(3, 1, Some(2), false);
    colorless_is_planned_with_an_untapped_forest_for_the_first_player: planning_with_a_forest(4, 0, None, true);
    colorless_is_planned_with_an_untapped_forest_for_the_second_player: planning_with_a_forest(4, 1, None, true);
    colorless_is_planned_with_a_tapped_forest_for_the_first_player: planning_with_a_forest(4, 0, None, false);
    colorless_is_planned_with_a_tapped_forest_for_the_second_player: planning_with_a_forest(4, 1, None, false);
    two_green_are_planned_with_an_untapped_forest_and_no_green_floating_for_the_first_player: planning_with_a_forest(5, 0, Some(0), true);
    two_green_are_planned_with_an_untapped_forest_and_no_green_floating_for_the_second_player: planning_with_a_forest(5, 1, Some(0), true);
    two_green_are_planned_with_an_untapped_forest_and_one_green_floating_for_the_first_player: planning_with_a_forest(5, 0, Some(1), true);
    two_green_are_planned_with_an_untapped_forest_and_one_green_floating_for_the_second_player: planning_with_a_forest(5, 1, Some(1), true);
    two_green_are_planned_with_an_untapped_forest_and_two_green_floating_for_the_first_player: planning_with_a_forest(5, 0, Some(2), true);
    two_green_are_planned_with_an_untapped_forest_and_two_green_floating_for_the_second_player: planning_with_a_forest(5, 1, Some(2), true);
    two_green_are_planned_with_a_tapped_forest_and_no_green_floating_for_the_first_player: planning_with_a_forest(5, 0, Some(0), false);
    two_green_are_planned_with_a_tapped_forest_and_no_green_floating_for_the_second_player: planning_with_a_forest(5, 1, Some(0), false);
    two_green_are_planned_with_a_tapped_forest_and_one_green_floating_for_the_first_player: planning_with_a_forest(5, 0, Some(1), false);
    two_green_are_planned_with_a_tapped_forest_and_one_green_floating_for_the_second_player: planning_with_a_forest(5, 1, Some(1), false);
    two_green_are_planned_with_a_tapped_forest_and_two_green_floating_for_the_first_player: planning_with_a_forest(5, 0, Some(2), false);
    two_green_are_planned_with_a_tapped_forest_and_two_green_floating_for_the_second_player: planning_with_a_forest(5, 1, Some(2), false);
    one_is_paid_with_a_tapped_forest_for_the_first_player: paying_with_a_forest(0, 0, None, false);
    one_is_paid_with_a_tapped_forest_for_the_second_player: paying_with_a_forest(0, 1, None, false);
    green_is_paid_with_an_untapped_forest_and_no_green_floating_for_the_first_player: paying_with_a_forest(1, 0, Some(0), true);
    green_is_paid_with_an_untapped_forest_and_no_green_floating_for_the_second_player: paying_with_a_forest(1, 1, Some(0), true);
    green_is_paid_with_an_untapped_forest_and_one_green_floating_for_the_first_player: paying_with_a_forest(1, 0, Some(1), true);
    green_is_paid_with_an_untapped_forest_and_one_green_floating_for_the_second_player: paying_with_a_forest(1, 1, Some(1), true);
    green_is_paid_with_an_untapped_forest_and_two_green_floating_for_the_first_player: paying_with_a_forest(1, 0, Some(2), true);
    green_is_paid_with_an_untapped_forest_and_two_green_floating_for_the_second_player: paying_with_a_forest(1, 1, Some(2), true);
    green_is_paid_with_a_tapped_forest_and_no_green_floating_for_the_first_player: paying_with_a_forest(1, 0, Some(0), false);
    green_is_paid_with_a_tapped_forest_and_no_green_floating_for_the_second_player: paying_with_a_forest(1, 1, Some(0), false);
    green_is_paid_with_a_tapped_forest_and_one_green_floating_for_the_first_player: paying_with_a_forest(1, 0, Some(1), false);
    green_is_paid_with_a_tapped_forest_and_one_green_floating_for_the_second_player: paying_with_a_forest(1, 1, Some(1), false);
    green_is_paid_with_a_tapped_forest_and_two_green_floating_for_the_first_player: paying_with_a_forest(1, 0, Some(2), false);
    green_is_paid_with_a_tapped_forest_and_two_green_floating_for_the_second_player: paying_with_a_forest(1, 1, Some(2), false);
    one_and_green_are_paid_with_an_untapped_forest_and_no_green_floating_for_the_first_player: paying_with_a_forest(2, 0, Some(0), true);
    one_and_green_are_paid_with_an_untapped_forest_and_no_green_floating_for_the_second_player: paying_with_a_forest(2, 1, Some(0), true);
    one_and_green_are_paid_with_a_tapped_forest_and_no_green_floating_for_the_first_player: paying_with_a_forest(2, 0, Some(0), false);
    one_and_green_are_paid_with_a_tapped_forest_and_no_green_floating_for_the_second_player: paying_with_a_forest(2, 1, Some(0), false);
    one_and_green_are_paid_with_a_tapped_forest_and_one_green_floating_for_the_first_player: paying_with_a_forest(2, 0, Some(1), false);
    one_and_green_are_paid_with_a_tapped_forest_and_one_green_floating_for_the_second_player: paying_with_a_forest(2, 1, Some(1), false);
    one_and_green_are_paid_with_a_tapped_forest_and_two_green_floating_for_the_first_player: paying_with_a_forest(2, 0, Some(2), false);
    one_and_green_are_paid_with_a_tapped_forest_and_two_green_floating_for_the_second_player: paying_with_a_forest(2, 1, Some(2), false);
    two_white_are_paid_with_an_untapped_forest_and_no_white_floating_for_the_first_player: paying_with_a_forest(3, 0, Some(0), true);
    two_white_are_paid_with_an_untapped_forest_and_no_white_floating_for_the_second_player: paying_with_a_forest(3, 1, Some(0), true);
    two_white_are_paid_with_an_untapped_forest_and_one_white_floating_for_the_first_player: paying_with_a_forest(3, 0, Some(1), true);
    two_white_are_paid_with_an_untapped_forest_and_one_white_floating_for_the_second_player: paying_with_a_forest(3, 1, Some(1), true);
    two_white_are_paid_with_an_untapped_forest_and_two_white_floating_for_the_first_player: paying_with_a_forest(3, 0, Some(2), true);
    two_white_are_paid_with_an_untapped_forest_and_two_white_floating_for_the_second_player: paying_with_a_forest(3, 1, Some(2), true);
    two_white_are_paid_with_a_tapped_forest_and_no_white_floating_for_the_first_player: paying_with_a_forest(3, 0, Some(0), false);
    two_white_are_paid_with_a_tapped_forest_and_no_white_floating_for_the_second_player: paying_with_a_forest(3, 1, Some(0), false);
    two_white_are_paid_with_a_tapped_forest_and_one_white_floating_for_the_first_player: paying_with_a_forest(3, 0, Some(1), false);
    two_white_are_paid_with_a_tapped_forest_and_one_white_floating_for_the_second_player: paying_with_a_forest(3, 1, Some(1), false);
    two_white_are_paid_with_a_tapped_forest_and_two_white_floating_for_the_first_player: paying_with_a_forest(3, 0, Some(2), false);
    two_white_are_paid_with_a_tapped_forest_and_two_white_floating_for_the_second_player: paying_with_a_forest(3, 1, Some(2), false);
    colorless_is_paid_with_an_untapped_forest_for_the_first_player: paying_with_a_forest(4, 0, None, true);
    colorless_is_paid_with_an_untapped_forest_for_the_second_player: paying_with_a_forest(4, 1, None, true);
    colorless_is_paid_with_a_tapped_forest_for_the_first_player: paying_with_a_forest(4, 0, None, false);
    colorless_is_paid_with_a_tapped_forest_for_the_second_player: paying_with_a_forest(4, 1, None, false);
    two_green_are_paid_with_an_untapped_forest_and_no_green_floating_for_the_first_player: paying_with_a_forest(5, 0, Some(0), true);
    two_green_are_paid_with_an_untapped_forest_and_no_green_floating_for_the_second_player: paying_with_a_forest(5, 1, Some(0), true);
    two_green_are_paid_with_an_untapped_forest_and_one_green_floating_for_the_first_player: paying_with_a_forest(5, 0, Some(1), true);
    two_green_are_paid_with_an_untapped_forest_and_one_green_floating_for_the_second_player: paying_with_a_forest(5, 1, Some(1), true);
    two_green_are_paid_with_an_untapped_forest_and_two_green_floating_for_the_first_player: paying_with_a_forest(5, 0, Some(2), true);
    two_green_are_paid_with_an_untapped_forest_and_two_green_floating_for_the_second_player: paying_with_a_forest(5, 1, Some(2), true);
    two_green_are_paid_with_a_tapped_forest_and_no_green_floating_for_the_first_player: paying_with_a_forest(5, 0, Some(0), false);
    two_green_are_paid_with_a_tapped_forest_and_no_green_floating_for_the_second_player: paying_with_a_forest(5, 1, Some(0), false);
    two_green_are_paid_with_a_tapped_forest_and_one_green_floating_for_the_first_player: paying_with_a_forest(5, 0, Some(1), false);
    two_green_are_paid_with_a_tapped_forest_and_one_green_floating_for_the_second_player: paying_with_a_forest(5, 1, Some(1), false);
    two_green_are_paid_with_a_tapped_forest_and_two_green_floating_for_the_first_player: paying_with_a_forest(5, 0, Some(2), false);
    two_green_are_paid_with_a_tapped_forest_and_two_green_floating_for_the_second_player: paying_with_a_forest(5, 1, Some(2), false);
}

/// A proof above for each player, with [`each_cost`]'s stand-ins.
macro_rules! each_player {
    ($($name:ident: $proof:ident($who:literal);)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        /// The proof above it, for one player.
        fn $name() {
            $proof(PlayerId($who));
        }
    )*};
}

each_player! {
    the_forests_ability_taps_it_and_adds_green_for_the_first_player: activating_the_forest(0);
    the_forests_ability_taps_it_and_adds_green_for_the_second_player: activating_the_forest(1);
}

/// [`paying_with_a_forest_holding`] for each cost, player and pool
/// amounts given, with [`each_forest`]'s stand-ins.
macro_rules! each_holding {
    ($($name:ident: ($which:literal, $who:literal, $held:expr);)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::engine::activate_mana_source_reserving, forest_activation)]
        /// The proof above it, for one cost, one player and one pool.
        fn $name() {
            paying_with_a_forest_holding($which, PlayerId($who), &$held);
        }
    )*};
}

each_holding! {
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_one_green_and_no_colorless_for_the_first_player: (2, 0, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_one_green_and_one_colorless_for_the_first_player: (2, 0, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_one_green_and_two_colorless_for_the_first_player: (2, 0, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_one_green_and_no_colorless_for_the_first_player: (2, 0, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_one_green_and_one_colorless_for_the_first_player: (2, 0, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_one_green_and_two_colorless_for_the_first_player: (2, 0, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_one_green_and_no_colorless_for_the_first_player: (2, 0, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_one_green_and_one_colorless_for_the_first_player: (2, 0, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_one_green_and_two_colorless_for_the_first_player: (2, 0, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_two_green_and_no_colorless_for_the_first_player: (2, 0, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_two_green_and_one_colorless_for_the_first_player: (2, 0, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_two_green_and_two_colorless_for_the_first_player: (2, 0, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_two_green_and_no_colorless_for_the_first_player: (2, 0, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_two_green_and_one_colorless_for_the_first_player: (2, 0, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_two_green_and_two_colorless_for_the_first_player: (2, 0, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_two_green_and_no_colorless_for_the_first_player: (2, 0, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_two_green_and_one_colorless_for_the_first_player: (2, 0, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_two_green_and_two_colorless_for_the_first_player: (2, 0, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_one_green_and_no_colorless_for_the_second_player: (2, 1, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_one_green_and_one_colorless_for_the_second_player: (2, 1, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_one_green_and_two_colorless_for_the_second_player: (2, 1, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_one_green_and_no_colorless_for_the_second_player: (2, 1, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_one_green_and_one_colorless_for_the_second_player: (2, 1, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_one_green_and_two_colorless_for_the_second_player: (2, 1, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_one_green_and_no_colorless_for_the_second_player: (2, 1, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_one_green_and_one_colorless_for_the_second_player: (2, 1, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_one_green_and_two_colorless_for_the_second_player: (2, 1, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_two_green_and_no_colorless_for_the_second_player: (2, 1, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_two_green_and_one_colorless_for_the_second_player: (2, 1, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_no_white_two_green_and_two_colorless_for_the_second_player: (2, 1, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_two_green_and_no_colorless_for_the_second_player: (2, 1, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_two_green_and_one_colorless_for_the_second_player: (2, 1, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_one_white_two_green_and_two_colorless_for_the_second_player: (2, 1, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_two_green_and_no_colorless_for_the_second_player: (2, 1, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_two_green_and_one_colorless_for_the_second_player: (2, 1, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_and_green_are_paid_with_an_untapped_forest_from_two_white_two_green_and_two_colorless_for_the_second_player: (2, 1, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_no_white_no_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 0), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_no_white_no_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 0), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_no_white_no_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 0), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_no_white_one_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_no_white_one_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_no_white_one_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_no_white_two_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_no_white_two_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_no_white_two_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_one_white_no_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 0), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_one_white_no_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 0), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_one_white_no_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 0), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_one_white_one_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_one_white_one_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_one_white_one_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_one_white_two_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_one_white_two_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_one_white_two_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_two_white_no_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 0), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_two_white_no_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 0), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_two_white_no_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 0), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_two_white_one_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_two_white_one_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_two_white_one_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_two_white_two_green_and_no_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_two_white_two_green_and_one_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_two_white_two_green_and_two_colorless_for_the_first_player: (0, 0, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_no_white_no_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 0), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_no_white_no_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 0), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_no_white_no_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 0), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_no_white_one_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_no_white_one_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_no_white_one_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_no_white_two_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_no_white_two_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_no_white_two_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 0), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_one_white_no_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 0), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_one_white_no_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 0), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_one_white_no_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 0), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_one_white_one_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_one_white_one_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_one_white_one_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_one_white_two_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_one_white_two_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_one_white_two_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 1), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_two_white_no_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 0), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_two_white_no_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 0), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_two_white_no_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 0), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_two_white_one_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_two_white_one_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_two_white_one_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 1), (ManaType::Colorless, 2)]);
    one_is_paid_with_an_untapped_forest_from_two_white_two_green_and_no_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 0)]);
    one_is_paid_with_an_untapped_forest_from_two_white_two_green_and_one_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 1)]);
    one_is_paid_with_an_untapped_forest_from_two_white_two_green_and_two_colorless_for_the_second_player: (0, 1, [(ManaType::White, 2), (ManaType::Green, 2), (ManaType::Colorless, 2)]);
}

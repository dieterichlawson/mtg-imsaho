//! One combat damage step's damage (`combat::queue_damage_step`, CR 510.1,
//! 510.4-510.5, 702.4, 702.7, 702.19b): which creatures deal combat damage
//! in the step, to what, and how much, all of it queued to be dealt at
//! once (510.2; dealing the queue is `damage`'s proof). How a blocked
//! attacker's damage is divided among its blockers is
//! `combat::divide_combat_damage`, the division kernel proved sound and
//! complete (`mtg_kernel::combat::divide`), standing in here as any
//! division it is proved to give; whether a creature deals damage in the
//! step (`combat::deals_in_step`) and the record that it dealt first-strike
//! damage (`combat::mark_dealt_first_strike`) are each their own proof
//! below; whether a creature has trample or deathtouch, and its power and
//! toughness, are any answer.

use crate::cards::CardRegistry;
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{asked_each, card_proof, Act, Blank};
use crate::state::{CombatState, GameState};
use crate::types::{Keyword, Zone};

/// Each object's keywords asked (first strike, double strike, trample,
/// deathtouch), power and toughness: any answer, the same each time.
static mut KEYWORDS: [[Option<bool>; 4]; 5] = [[None; 4]; 5];
static mut POWER: [Option<i32>; 5] = [None; 5];
static mut TOUGHNESS: [Option<i32>; 5] = [None; 5];

fn index(id: ObjectId) -> usize {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=5).contains(&i), "an object this proof does not make");
    i - 1
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    let k = match keyword {
        Keyword::FirstStrike => 0,
        Keyword::DoubleStrike => 1,
        Keyword::Trample => 2,
        Keyword::Deathtouch => 3,
        _ => panic!("a keyword the step does not ask"),
    };
    // SAFETY: a proof runs on one thread, and only this function names KEYWORDS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(KEYWORDS) };
    *answers[index(id)][k].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::effective_power`: any power from -1 to 3.
#[allow(dead_code)] // named only by `kani::stub`
fn effective_power(_: &GameState, id: ObjectId, _: &CardRegistry) -> Option<i32> {
    // SAFETY: as in `has_keyword`, for POWER.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(POWER) };
    Some(*answers[index(id)].get_or_insert_with(|| {
        let p: i32 = kani::any();
        kani::assume((-1..=3).contains(&p));
        p
    }))
}

/// Stands in for `GameState::effective_toughness`: any toughness from 0
/// to 3 (what lethal damage is made of it is the division's).
#[allow(dead_code)] // named only by `kani::stub`
fn effective_toughness(_: &GameState, id: ObjectId, _: &CardRegistry) -> Option<i32> {
    // SAFETY: as in `has_keyword`, for TOUGHNESS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(TOUGHNESS) };
    Some(*answers[index(id)].get_or_insert_with(|| {
        let t: i32 = kani::any();
        kani::assume((0..=3).contains(&t));
        t
    }))
}

/// The divisions asked for, by attacker (1 or 2): the power, the blockers
/// it was divided among, in order, whether with trample, and the answer.
#[derive(Clone, Copy)]
struct Asked {
    power: u32,
    blockers: [ObjectId; 2],
    n: usize,
    trample: bool,
    amounts: [u32; 2],
    overflow: u32,
    open: Option<(ObjectId, u32, u32)>,
}
static mut DIVISIONS: [Option<Asked>; 2] = [None; 2];

/// Stands in for `combat::divide_combat_damage`: any division the kernel
/// is proved to give (`mtg_kernel::combat::legal_division`): the amounts
/// and the overflow add up to the power, or to none if no blocker is left
/// and the attacker has no trample (CR 510.1c); a blocker is assigned
/// anything only once each before it has lethal damage; and anything
/// tramples over only with trample, every blocker assigned lethal damage
/// (702.19b). Recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn divide(power: u32, blockers: &[(ObjectId, u32)], trample: bool, _: &[(ObjectId, u32)]) -> crate::combat::DamageDivision {
    assert!(blockers.len() <= 2, "more blockers than the board makes");
    let mut asked = Asked { power, blockers: [ObjectId(0); 2], n: blockers.len(), trample, amounts: [0; 2], overflow: kani::any(), open: None };
    kani::assume(asked.overflow <= 3);
    let mut to_blockers = Vec::with_capacity(2);
    let mut total = asked.overflow;
    for (k, &(b, _)) in blockers.iter().enumerate() {
        let a: u32 = kani::any();
        kani::assume(a <= 3);
        (asked.blockers[k], asked.amounts[k]) = (b, a);
        total += a;
        to_blockers.push((b, a));
    }
    kani::assume(total == if blockers.is_empty() && !trample { 0 } else { power });
    let lethal = |k: usize| asked.amounts[k] >= blockers[k].1;
    kani::assume(!(blockers.len() == 2 && asked.amounts[1] > 0) || lethal(0));
    kani::assume(asked.overflow == 0 || (trample && (0..blockers.len()).all(lethal)));
    // SAFETY: as in `has_keyword`, for DIVISIONS. Asked once per attacker
    // of the board, in the caller's order, which the proof follows.
    let divisions = unsafe { &mut *std::ptr::addr_of_mut!(DIVISIONS) };
    let slot = divisions.iter_mut().find(|d| d.is_none()).expect("a division per attacker");
    *slot = Some(asked);
    crate::combat::DamageDivision { to_blockers, overflow: asked.overflow, open: None }
}

/// Stands in for `combat::divide_combat_damage` where the proof is of the
/// question it raises: recorded as [`divide`] is, the amounts it gives
/// unread, and any blocker of those divided among left open, with any
/// range of two amounts or more, or none.
#[allow(dead_code)] // named only by `kani::stub`
fn divide_open(power: u32, blockers: &[(ObjectId, u32)], trample: bool, _: &[(ObjectId, u32)]) -> crate::combat::DamageDivision {
    assert!(blockers.len() <= 2, "more blockers than the board makes");
    let mut asked = Asked { power, blockers: [ObjectId(0); 2], n: blockers.len(), trample, amounts: [0; 2], overflow: 0, open: None };
    for (k, &(b, _)) in blockers.iter().enumerate() {
        asked.blockers[k] = b;
    }
    if !blockers.is_empty() && kani::any() {
        let k = usize::from(blockers.len() == 2 && kani::any::<bool>());
        let (lo, hi): (u32, u32) = (kani::any(), kani::any());
        kani::assume(lo < hi && hi <= 3);
        asked.open = Some((blockers[k].0, lo, hi));
    }
    // SAFETY: as in `divide`.
    let divisions = unsafe { &mut *std::ptr::addr_of_mut!(DIVISIONS) };
    let slot = divisions.iter_mut().find(|d| d.is_none()).expect("a division per attacker");
    *slot = Some(asked);
    crate::combat::DamageDivision { to_blockers: Vec::new(), overflow: 0, open: asked.open }
}

/// Whether each object deals combat damage in this step: any answer (the
/// rule is `deals_in_step`'s, proved below); and the objects marked as
/// having dealt first-strike damage, in order.
static mut DEALS: [Option<bool>; 5] = [None; 5];
static mut MARKED: [ObjectId; 4] = [ObjectId(0); 4];
static mut MARKED_N: usize = 0;

/// Stands in for `combat::deals_in_step`: any answer, the same each time.
#[allow(dead_code)] // named only by `kani::stub`
fn deals(_: &GameState, id: ObjectId, _: bool, _: &CardRegistry) -> bool {
    // SAFETY: as in `has_keyword`, for DEALS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(DEALS) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `combat::mark_dealt_first_strike` (its proof below):
/// recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn mark(_: &mut GameState, id: ObjectId) {
    // SAFETY: as in `has_keyword`, for MARKED and MARKED_N.
    let (list, n) = unsafe { (&mut *std::ptr::addr_of_mut!(MARKED), &mut *std::ptr::addr_of_mut!(MARKED_N)) };
    list[*n] = id;
    *n += 1;
}

/// A creature's power, as combat damage: none below 0.
fn power(state: &GameState, registry: &CardRegistry, id: u64) -> u32 {
    u32::try_from(effective_power(state, ObjectId(id), registry).expect("a power").max(0)).expect("small")
}

/// The board of [`dealing`] and [`asking`]: attackers 1 and 2 (the active
/// player's), blockers 3 and 4 (the defending player's) and a planeswalker
/// 5 (the defending player's), each on the battlefield or not, in a combat
/// as `attacking`, `walker_of`, `blocks` and `order` say; and a
/// first-strike step or not. Which creature deals damage in the step is
/// `deals_in_step`'s, any answer here: with each creature having dealt
/// first-strike damage or not, the combat's set of them was built every
/// way, and copied, and ran symbolic execution out of memory.
struct Board {
    registry: CardRegistry,
    state: GameState,
    active: PlayerId,
    defending: PlayerId,
    on: [bool; 5],
    first_strike_step: bool,
    combat: CombatState,
}

fn board(attacking: &[u64], walker_of: Option<u64>, blocks: &[(u64, &[u64])], order: &[(u64, &[u64])]) -> Board {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let active = PlayerId(u8::from(kani::any::<bool>()));
    state.active_player = active;
    let defending = state.opponent(active);
    let mut on = [false; 5];
    for (i, o) in on.iter_mut().enumerate() {
        *o = kani::any();
        let controller = if i < 2 { active } else { defending };
        let zone = if *o { Zone::Battlefield } else { Zone::Graveyard };
        state.create_object(blank, controller, zone, Some(1), Some(1));
    }
    let first_strike_step: bool = kani::any();
    // Built twice, once for the game and once for the step to read, rather
    // than cloned: a copy of the combat's maps was most of what symbolic
    // execution followed.
    state.combat = Some(combat_of(attacking, walker_of, blocks, order, defending));
    let combat = combat_of(attacking, walker_of, blocks, order, defending);

    Board { registry, state, active, defending, on, first_strike_step, combat }
}

/// The combat of [`board`].
fn combat_of(attacking: &[u64], walker_of: Option<u64>, blocks: &[(u64, &[u64])], order: &[(u64, &[u64])], defending: PlayerId) -> CombatState {
    let mut combat = CombatState::default();
    for &a in attacking {
        combat.attackers.insert(ObjectId(a), defending);
        let declared: Vec<ObjectId> = blocks.iter().find(|&&(x, _)| x == a).map_or(&[][..], |&(_, b)| b)
            .iter().map(|&b| ObjectId(b)).collect();
        if !declared.is_empty() {
            combat.blocked_attackers.insert(ObjectId(a));
        }
        combat.blocker_assignments.insert(ObjectId(a), declared);
        if let Some(&(_, announced)) = order.iter().find(|&&(x, _)| x == a) {
            combat.damage_assignment_order.insert(ObjectId(a), announced.iter().map(|&b| ObjectId(b)).collect());
        }
        if walker_of == Some(a) {
            combat.planeswalker_defenders.insert(ObjectId(a), ObjectId(5));
        }
    }
    combat.chosen_damage.insert(ObjectId(1), Vec::new());
    combat
}

/// One combat damage step's damage. The board: attackers 1 and 2 (the
/// active player's), blockers 3 and 4 (the defending player's) and a
/// planeswalker 5 (the defending player's); each on the battlefield or
/// not; each in this step's combat or not, as `attacking`, `walker_of`
/// (the attacker at the planeswalker), `blocks` (each attacker's blockers
/// as declared) and `order` (the damage assignment order announced, empty
/// for none) say, constants of each harness. Each creature's first strike,
/// double strike, trample and power are any answer, as is whether it dealt
/// damage in a first-strike step already, and whether this step is one.
///
/// In this step, a creature deals combat damage if it has double strike,
/// or, in a first-strike step, first strike (702.7b, 702.4b), or, in a
/// regular step, dealt none in a first-strike step (510.4); a creature
/// dealing damage in a first-strike step is marked as having done so. Of
/// the attackers in combat and on the battlefield, in the board's order:
/// one unblocked deals damage equal to its power to the player, or the
/// planeswalker it attacks if that is still on the battlefield under the
/// defending player, and none if it is not (510.1a-b); one blocked (even
/// if none of its blockers is left, 509.1h, 510.1c) has each of its
/// blockers still on the battlefield, in the damage assignment order
/// announced, deal it damage equal to the blocker's power, and deals each
/// of them what the division gives it, and any overflow (trample) to what
/// it attacks as before (702.19b-c); the division is asked of exactly those
/// blockers, in that order, with the attacker's power (none if it deals
/// none) and whether it has trample. Nothing of no damage is queued, and
/// every event is combat damage. The step's chosen divisions are spent.
fn dealing(attacking: &[u64], walker_of: Option<u64>, blocks: &[(u64, &[u64])], order: &[(u64, &[u64])]) {
    let Board { registry, mut state, defending, on, first_strike_step, combat, .. } =
        board(attacking, walker_of, blocks, order);

    crate::combat::queue_damage_step(&mut state, &combat, &registry, first_strike_step);

    let deals = |id: u64| deals(&state, ObjectId(id), first_strike_step, &registry);
    let power = |id: u64| power(&state, &registry, id);
    // SAFETY: as in `has_keyword`.
    let divisions = unsafe { *std::ptr::addr_of!(DIVISIONS) };
    let mut expected = [(ObjectId(0), DamageTarget::Player(PlayerId(0)), 0u32); 8];
    let mut n = 0;
    let mut push = |source: u64, target: DamageTarget, amount: u32| {
        if amount > 0 {
            expected[n] = (ObjectId(source), target, amount);
            n += 1;
        }
    };
    let (mut marked, mut m) = ([ObjectId(0); 4], 0);
    let mut divided = 0;
    for &a in attacking {
        if !on[index(ObjectId(a))] {
            continue;
        }
        let attacker_deals = deals(a);
        if attacker_deals && first_strike_step {
            marked[m] = ObjectId(a);
            m += 1;
        }
        let p = if attacker_deals { power(a) } else { 0 };
        let at = match walker_of {
            Some(w) if w == a => on[4].then_some(DamageTarget::Object(ObjectId(5))),
            _ => Some(DamageTarget::Player(defending)),
        };
        let declared = blocks.iter().find(|&&(x, _)| x == a).map_or(&[][..], |&(_, b)| b);
        if declared.is_empty() {
            if let Some(t) = at {
                push(a, t, p);
            }
            continue;
        }
        let announced = order.iter().find(|&&(x, _)| x == a).map_or(declared, |&(_, o)| o);
        let d = divisions[divided].expect("the attacker's division asked");
        divided += 1;
        let mut live = 0;
        for &b in announced {
            if !on[index(ObjectId(b))] {
                continue;
            }
            assert!(live < d.n && d.blockers[live] == ObjectId(b));
            if deals(b) {
                if first_strike_step {
                    marked[m] = ObjectId(b);
                    m += 1;
                }
                push(b, DamageTarget::Object(ObjectId(a)), power(b));
            }
            push(a, DamageTarget::Object(ObjectId(b)), d.amounts[live]);
            live += 1;
        }
        assert!(d.n == live && d.power == p && d.trample == has_keyword(&state, ObjectId(a), Keyword::Trample, &registry));
        if let Some(t) = at {
            push(a, t, d.overflow);
        }
    }
    assert!(divisions[divided..].iter().all(Option::is_none));
    assert!(asked_each(n, |k| Act::Queue { source: expected[k].0, target: expected[k].1, amount: expected[k].2, kind: DamageKind::Combat }));
    // SAFETY: as in `has_keyword`.
    let (recorded, k) = unsafe { (*std::ptr::addr_of!(MARKED), *std::ptr::addr_of!(MARKED_N)) };
    assert!(k == m && recorded[..m] == marked[..m]);
    assert!(state.combat.as_ref().expect("a combat").chosen_damage.is_empty());
    std::mem::forget((registry, state, combat));
}

/// The proofs of a combat damage step, a shape of combat each.
macro_rules! each_combat {
    ($($name:ident: $attacking:expr, $walker:expr, $blocks:expr, $order:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
            #[kani::stub(crate::state::GameState::effective_power, effective_power)]
            #[kani::stub(crate::state::GameState::effective_toughness, effective_toughness)]
            #[kani::stub(crate::combat::divide_combat_damage, divide)]
            #[kani::stub(crate::combat::deals_in_step, deals)]
            #[kani::stub(crate::combat::mark_dealt_first_strike, mark)]
            /// [`dealing`] for the combat of its name.
            fn $name() {
                dealing(&$attacking, $walker, &$blocks, &$order);
            }
        }
    )*};
}

each_combat! {
    an_unblocked_attacker_deals_its_power_to_the_player: [1], None, [], [];
    an_unblocked_attacker_at_a_planeswalker_deals_its_power_to_it_while_it_is_there: [1], Some(1), [], [];
    a_blocked_attacker_and_its_blocker_deal_damage_to_each_other: [1], None, [(1, &[3][..])], [];
    a_blocked_attacker_deals_damage_to_its_blockers_in_the_order_announced: [1], None, [(1, &[3, 4][..])], [(1, &[4, 3][..])];
    a_blocked_attacker_at_a_planeswalker_tramples_over_onto_it: [1], Some(1), [(1, &[3][..])], [];
    two_attackers_deal_their_damage_in_the_boards_order: [1, 2], None, [(2, &[4][..])], [];
}

/// The question a combat damage step asks first (`combat::
/// ask_next_damage_division`, CR 510.1c-d): of the attackers in combat and
/// on the battlefield that deal combat damage in this step and have a
/// blocker still on the battlefield, in the board's order, the division of
/// each is asked of exactly those blockers, in the damage assignment order
/// announced, with its power and whether it has trample; the first whose
/// division leaves a blocker open has the attacking player asked how much
/// of the damage goes to that blocker, the range the division gives, in
/// this step, and no attacker after it is asked of; with none, nothing is
/// asked. On the board of [`dealing`], each division leaving any blocker
/// open with any range, or none.
fn asking(attacking: &[u64], blocks: &[(u64, &[u64])], order: &[(u64, &[u64])]) {
    let Board { registry, mut state, active, on, first_strike_step, .. } = board(attacking, None, blocks, order);

    let raised = crate::combat::ask_next_damage_division(&mut state, &registry, first_strike_step);

    // SAFETY: as in `has_keyword`.
    let divisions = unsafe { *std::ptr::addr_of!(DIVISIONS) };
    let mut divided = 0;
    let mut question = None;
    for &a in attacking {
        if !on[index(ObjectId(a))] || !deals(&state, ObjectId(a), first_strike_step, &registry) {
            continue;
        }
        let declared = blocks.iter().find(|&&(x, _)| x == a).map_or(&[][..], |&(_, b)| b);
        let announced = order.iter().find(|&&(x, _)| x == a).map_or(declared, |&(_, o)| o);
        let live = announced.iter().filter(|&&b| on[index(ObjectId(b))]).count();
        if live == 0 {
            continue;
        }
        let d = divisions[divided].expect("the attacker's division asked");
        divided += 1;
        let mut k = 0;
        for &b in announced {
            if on[index(ObjectId(b))] {
                assert!(d.blockers[k] == ObjectId(b));
                k += 1;
            }
        }
        assert!(d.n == live && d.power == power(&state, &registry, a)
            && d.trample == has_keyword(&state, ObjectId(a), Keyword::Trample, &registry));
        if let Some(open) = d.open {
            question = Some((ObjectId(a), open));
            break;
        }
    }
    assert!(divisions[divided..].iter().all(Option::is_none));
    match question {
        Some((a, (b, lo, hi))) => {
            assert!(raised && matches!(&state.awaiting_action, Some(crate::state::AwaitingAction::ResolutionChoice {
                player, source,
                choice: crate::state::ResolutionChoiceKind::AssignCombatDamage { attacker, blocker, min, max, first_strike_only, .. },
            }) if *player == active && *source == a && *attacker == a && *blocker == b && *min == lo && *max == hi
                && *first_strike_only == first_strike_step));
        }
        None => {
            assert!(!raised && state.awaiting_action.is_none());
        }
    }
    std::mem::forget((registry, state));
}

/// The proofs of the question a combat damage step asks.
macro_rules! each_question {
    ($($name:ident: $attacking:expr, $blocks:expr, $order:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
            #[kani::stub(crate::state::GameState::effective_power, effective_power)]
            #[kani::stub(crate::state::GameState::effective_toughness, effective_toughness)]
            #[kani::stub(crate::combat::divide_combat_damage, divide_open)]
            #[kani::stub(crate::combat::deals_in_step, deals)]
            /// [`asking`] for the combat of its name.
            fn $name() {
                asking(&$attacking, &$blocks, &$order);
            }
        }
    )*};
}

each_question! {
    no_division_is_asked_of_an_unblocked_attacker: [1], [], [];
    a_division_is_asked_of_an_attacker_blocked_by_two_in_the_order_announced: [1], [(1, &[3, 4][..])], [(1, &[4, 3][..])];
    the_first_attacker_whose_division_is_open_is_asked_and_none_after_it: [1, 2], [(1, &[3][..]), (2, &[4][..])], [];
}

card_proof! {
    #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
    /// Whether a creature deals combat damage in this step (`combat::
    /// deals_in_step`, CR 510.4, 702.4b-c, 702.7b-c): in a first-strike
    /// step, exactly if it has first strike or double strike; in a regular
    /// step, exactly if it has double strike or dealt no damage in a
    /// first-strike step of this combat (one gaining first strike after it
    /// still deals its damage). For a creature with first strike or not
    /// and double strike or not, having dealt first-strike damage or not,
    /// in a combat or none, in either step.
    fn a_creature_deals_combat_damage_in_the_steps_the_rules_give_it() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let id = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
        let (in_combat, dealt): (bool, bool) = (kani::any(), kani::any());
        if in_combat {
            let mut combat = CombatState::default();
            if dealt {
                combat.dealt_first_strike.insert(id);
            }
            state.combat = Some(combat);
        }
        let first_strike_step: bool = kani::any();

        let deals_now = crate::combat::deals_in_step(&state, id, first_strike_step, &registry);

        let double = has_keyword(&state, id, Keyword::DoubleStrike, &registry);
        let expected = if first_strike_step {
            double || has_keyword(&state, id, Keyword::FirstStrike, &registry)
        } else {
            double || !(in_combat && dealt)
        };
        assert!(deals_now == expected);
        std::mem::forget((registry, state));
    }
}

/// Marking a creature as having dealt first-strike damage (`combat::
/// mark_dealt_first_strike`, CR 510.4): with a combat, it is among those
/// that did, with the others kept; with none, nothing is done. For a
/// combat or none in which creature 2 has dealt it already, and `id` a
/// constant of each harness: inserted into the combat's set at an id the
/// solver leaves open, it ran symbolic execution out of memory.
fn marking(id: ObjectId) {
    let mut state = crate::proofs::cards::new_game();
    let in_combat: bool = kani::any();
    if in_combat {
        let mut combat = CombatState::default();
        combat.dealt_first_strike.insert(ObjectId(2));
        state.combat = Some(combat);
    }

    crate::combat::mark_dealt_first_strike(&mut state, id);

    match &state.combat {
        Some(c) => {
            assert!(in_combat && c.dealt_first_strike.contains(&id) && c.dealt_first_strike.contains(&ObjectId(2))
                && c.dealt_first_strike.len() == if id == ObjectId(2) { 1 } else { 2 });
        }
        None => {
            assert!(!in_combat);
        }
    }
    std::mem::forget(state);
}

card_proof! {
    /// [`marking`] of a creature not marked yet.
    fn a_creature_dealing_first_strike_damage_is_marked_as_having_dealt_it() {
        marking(ObjectId(1));
    }
}

card_proof! {
    /// [`marking`] of a creature marked already.
    fn a_creature_marked_as_having_dealt_first_strike_damage_is_marked_once() {
        marking(ObjectId(2));
    }
}

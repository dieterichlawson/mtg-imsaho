//! Declaring blockers (`engine::actions::combat::declare_blockers`,
//! `combat::declare_blockers_with_registry` and `combat::declare_blockers`,
//! CR 509.1-2): which of the blocks a player submits stand, how they are
//! recorded, and what the declaration does after.
//!
//! Which pairs a declaration may hold is `mtg_kernel::blocks::judge_blocks`,
//! proved in Verus to accept exactly a legal declaration (509.1a-b); who
//! may block whom is `combat::can_block_attacker` (`blocking`) and how many
//! must block an attacker `combat::minimum_blockers`, each any answer here.
//! These are of how the declaration reads the board into the kernel's terms
//! and acts on its answer.

// The announcement takes an attacker at a time until none is left to ask
// about: each pass places a blocker or returns, so with the board's three
// blockers at most four passes, each over the board's two attackers.
// loop bound: combat::announce_damage_assignment_order 5
// loop bound: combat::next_unordered_attacker 3

use crate::proofs::held::Held;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, in_room, Blank};
use crate::state::{CombatState, GameState};
use crate::types::Zone;

/// Whether each of the two blockers (3 and 4) may block each of the two
/// attackers (1 and 2), and the fewest creatures that may block each
/// attacker: any answer, the same each time.
static mut MAY_BLOCK: Held<[[Option<bool>; 2]; 2]> = Held::new([[None; 2]; 2], (&raw const MAY_BLOCK).cast());
static mut MINIMUM: Held<[Option<u32>; 2]> = Held::new([None; 2], (&raw const MINIMUM).cast());

fn attacker(id: ObjectId) -> usize {
    match id.0 {
        1 => 0,
        2 => 1,
        _ => panic!("an attacker this proof does not make"),
    }
}

fn blocker(id: ObjectId) -> usize {
    match id.0 {
        3 => 0,
        4 => 1,
        _ => panic!("a blocker this proof does not make"),
    }
}

/// Stands in for `combat::can_block_attacker`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn can_block_attacker(_: &GameState, b: ObjectId, a: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: a proof runs on one thread, and only this function names MAY_BLOCK.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(MAY_BLOCK.value) };
    *answers[blocker(b)][attacker(a)].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `combat::minimum_blockers`: one or two.
#[allow(dead_code)] // named only by `kani::stub`
fn minimum_blockers(_: &GameState, a: ObjectId, _: &CardRegistry) -> u32 {
    // SAFETY: as in `can_block_attacker`, for MINIMUM.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(MINIMUM.value) };
    *answers[attacker(a)].get_or_insert_with(|| 1 + u32::from(kani::any::<bool>()))
}

/// A combat of the active player's two creatures (1 and 2), each attacking
/// the defending player as `attacking` says (a constant of each harness:
/// left to the solver, the combat's maps were built every way and ran
/// symbolic execution out of memory), and two creatures (3 and 4), each
/// either player's.
fn board(attacking: [bool; 2]) -> (CardRegistry, GameState, PlayerId, [bool; 2], [PlayerId; 2]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let active = PlayerId(u8::from(kani::any::<bool>()));
    state.active_player = active;
    let defending = state.opponent(active);
    let mut combat = CombatState::default();
    for &a in &attacking {
        let id = state.create_object(blank, active, Zone::Battlefield, Some(1), Some(1));
        if a {
            combat.attackers.insert(id, defending);
            combat.blocker_assignments.insert(id, Vec::with_capacity(2));
        }
    }
    let mut controllers = [PlayerId(0); 2];
    for c in &mut controllers {
        *c = PlayerId(u8::from(kani::any::<bool>()));
        state.create_object(blank, *c, Zone::Battlefield, Some(1), Some(1));
    }
    state.combat = Some(combat);
    (registry, state, defending, attacking, controllers)
}

/// The blocks handed on to be recorded.
static mut RECORDED: Held<[(ObjectId, ObjectId); 4]> = Held::new([(ObjectId(0), ObjectId(0)); 4], (&raw const RECORDED).cast());
static mut RECORDED_N: Held<Option<usize>> = Held::new(None, (&raw const RECORDED_N).cast());

/// Stands in for `combat::declare_blockers`: records what it is handed.
#[allow(dead_code)] // named only by `kani::stub`
fn record_blocks(_: &mut GameState, blocks: &[(ObjectId, ObjectId)]) {
    // SAFETY: as in `can_block_attacker`, for RECORDED and RECORDED_N.
    let (list, n) = unsafe { (&mut *std::ptr::addr_of_mut!(RECORDED.value), &mut *std::ptr::addr_of_mut!(RECORDED_N.value)) };
    assert!(n.is_none(), "blocks recorded twice");
    // Element by element: `copy_from_slice` is a byte copy, and CBMC's,
    // through a pointer that may be to more than one buffer, kept none of
    // what it copied, so a block judged to stand read back as (0, 0).
    for (k, &pair) in blocks.iter().enumerate() {
        list[k] = pair;
    }
    *n = Some(blocks.len());
}

/// Which of a declaration's blocks stand (`declare_blockers_with_registry`,
/// CR 509.1a-b): the board read into the kernel's terms, each blocker and
/// attacker numbered in the order first named, a blocker allowed to block
/// an attacker exactly when the attacker is attacking, the blocker is the
/// defending player's and the rule (`can_block_attacker`) lets it, and
/// each attacker's fewest blockers `minimum_blockers`; the blocks recorded
/// are exactly those the kernel accepts, in the order submitted. For each
/// blocker either player's, each question any answer, and `blocks` and
/// which attackers attack constants of each harness. The declaration is
/// handed over in an array of room (`cards::in_room`): as an array of its
/// own length, none declared ran out of 12 GB.
fn deciding<const N: usize>(blocks: [(u64, u64); N], attacking: [bool; 2]) {
    let (registry, mut state, defending, attacking, controllers) = board(attacking);
    let room: [(ObjectId, ObjectId); 4] = in_room(&blocks.map(|(b, a)| (ObjectId(b), ObjectId(a))), (ObjectId(0), ObjectId(0)));
    let submitted = &room[..N];

    crate::combat::declare_blockers_with_registry(&mut state, submitted, &registry);

    // The terms, each numbered in the order first named.
    let (mut bs, mut as_) = (Vec::with_capacity(N), Vec::with_capacity(N));
    fn number(list: &mut Vec<ObjectId>, id: ObjectId) -> usize {
        match list.iter().position(|&x| x == id) {
            Some(i) => i,
            None => {
                list.push(id);
                list.len() - 1
            }
        }
    }
    let pairs: Vec<(usize, usize)> = submitted.iter().map(|&(b, a)| (number(&mut bs, b), number(&mut as_, a))).collect();
    let legal: Vec<Vec<bool>> = bs.iter().map(|&b| as_.iter().map(|&a| {
        attacking[attacker(a)] && controllers[blocker(b)] == defending && can_block_attacker(&state, b, a, &registry)
    }).collect()).collect();
    let minimum: Vec<u32> = as_.iter().map(|&a| minimum_blockers(&state, a, &registry)).collect();
    let verdicts = mtg_kernel::blocks::judge_blocks(&pairs, &legal, &minimum);
    let mut n = 0;
    // SAFETY: as in `can_block_attacker`.
    let (list, m) = unsafe { (&*std::ptr::addr_of!(RECORDED.value), *std::ptr::addr_of!(RECORDED_N.value)) };
    for (k, &pair) in submitted.iter().enumerate() {
        if matches!(verdicts[k], mtg_kernel::blocks::Verdict::Accepted) {
            assert!(list[n] == pair);
            n += 1;
        }
    }
    assert!(m == Some(n));
    std::mem::forget((registry, state, bs, as_, pairs, legal, minimum, verdicts));
}

/// The proofs of which blocks stand, the blocks recorded as handed on.
macro_rules! each_block {
    ($($name:ident: $blocks:expr, $attacking:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::combat::can_block_attacker, can_block_attacker)]
            #[kani::stub(crate::combat::minimum_blockers, minimum_blockers)]
            #[kani::stub(crate::combat::declare_blockers, record_blocks)]
            /// [`deciding`] for the blocks of its name.
            fn $name() {
                deciding($blocks, $attacking);
            }
        }
    )*};
}

each_block! {
    no_blocks_stand_of_none_declared: [], [true, true];
    one_block_stands_as_the_rules_allow: [(3, 1)], [true, true];
    a_block_of_a_creature_not_attacking_does_not_stand: [(3, 2)], [true, false];
    two_blockers_on_one_attacker_stand_as_the_rules_allow: [(3, 1), (4, 1)], [true, true];
    one_blocker_on_two_attackers_stands_on_one_at_most: [(3, 1), (3, 2)], [true, true];
    one_block_declared_twice_stands_once_at_most: [(3, 1), (3, 1)], [true, true];
    two_blockers_on_two_attackers_stand_as_the_rules_allow: [(4, 2), (3, 1)], [true, true];
}

/// The blocks recorded (`combat::declare_blockers`, CR 509.1h, 509.2):
/// each attacker attacking is blocked by exactly the blockers declared on
/// it, in order, and is blocked for the rest of the combat if any was
/// declared on it; an attacker not attacking is neither; a creature is a
/// blocking creature exactly if it was declared blocking an attacker
/// attacking (509.1g), which it stays whatever becomes of the attacker
/// (510.1d, `leaving_combat`); and the declaration's one event lists
/// exactly the blocks declared, in order.
/// For `blocks` and which attackers attack constants of each harness.
fn recording(blocks: &[(u64, u64)], attacking: [bool; 2]) {
    let (registry, mut state, _, attacking, _) = board(attacking);
    let declared: Vec<(ObjectId, ObjectId)> = blocks.iter().map(|&(b, a)| (ObjectId(b), ObjectId(a))).collect();
    let events = state.events.len();

    crate::combat::declare_blockers(&mut state, &declared);

    let combat = state.combat.as_ref().expect("a combat");
    for (i, &on) in attacking.iter().enumerate() {
        let a = ObjectId(i as u64 + 1);
        let on_it = blocks.iter().filter(|&&(_, x)| x == a.0).count();
        if on {
            let recorded = combat.blocker_assignments.get(&a).expect("an attacker's blockers");
            assert!(recorded.len() == on_it);
            let mut k = 0;
            for &(b, x) in blocks {
                if x == a.0 {
                    assert!(recorded[k] == ObjectId(b));
                    k += 1;
                }
            }
            assert!(combat.blocked_attackers.contains(&a) == (on_it > 0));
        } else {
            assert!(combat.blocker_assignments.get(&a).is_none() && !combat.blocked_attackers.contains(&a));
        }
    }
    for id in 1..=4u64 {
        let blocking = blocks.iter().any(|&(b, x)| b == id && attacking[usize::try_from(x).expect("small") - 1]);
        assert!(combat.blocking_creatures.contains(&ObjectId(id)) == blocking);
    }
    assert!(matches!(&state.events[events..], [GameEvent::BlockersDeclared { assignments }] if *assignments == declared));
    std::mem::forget((registry, state, declared));
}

/// The proofs of recording blocks.
macro_rules! each_block_recorded {
    ($($name:ident: $blocks:expr, $attacking:expr;)*) => {$(
        card_proof! {
            /// [`recording`] for the blocks of its name.
            fn $name() {
                recording(&$blocks, $attacking);
            }
        }
    )*};
}

each_block_recorded! {
    no_blocks_are_recorded_as_none: [], [true, true];
    one_block_is_recorded_on_its_attacker: [(3, 1)], [true, false];
    a_block_of_a_creature_not_attacking_is_listed_but_not_recorded_on_it: [(3, 2)], [true, false];
    two_blocks_on_one_attacker_are_recorded_in_order: [(4, 1), (3, 1)], [true, true];
    blocks_on_two_attackers_are_recorded_each_on_its_own: [(3, 2), (4, 1)], [true, true];
}

/// What the declaration did, in order: 1 for the blocks judged, with the
/// blocks submitted handed on; 2 for the damage assignment order
/// announced.
static mut STEPS: Held<[u8; 2]> = Held::new([0; 2], (&raw const STEPS).cast());
static mut STEPS_N: Held<usize> = Held::new(0, (&raw const STEPS_N).cast());
static mut HANDED: Held<bool> = Held::new(false, (&raw const HANDED).cast());

fn step(n: u8) {
    // SAFETY: as in `can_block_attacker`, for STEPS and STEPS_N.
    let (steps, k) = unsafe { (&mut *std::ptr::addr_of_mut!(STEPS.value), &mut *std::ptr::addr_of_mut!(STEPS_N.value)) };
    steps[*k] = n;
    *k += 1;
}

/// Stands in for `combat::declare_blockers_with_registry`: records that it
/// was handed the submission.
#[allow(dead_code)] // named only by `kani::stub`
fn judged(_: &mut GameState, blocks: &[(ObjectId, ObjectId)], _: &CardRegistry) {
    // SAFETY: as in `can_block_attacker`, for HANDED.
    unsafe { *std::ptr::addr_of_mut!(HANDED.value) = blocks == [(ObjectId(3), ObjectId(1)), (ObjectId(4), ObjectId(2))] };
    step(1);
}

/// Stands in for `combat::announce_damage_assignment_order`: recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn announced(_: &mut GameState, _: &CardRegistry) {
    step(2);
}

card_proof! {
    #[kani::stub(crate::combat::declare_blockers_with_registry, judged)]
    #[kani::stub(crate::combat::announce_damage_assignment_order, announced)]
    /// The declaration as a whole (CR 509.1, 509.2): the blocks submitted
    /// are judged and recorded as they say, the declaration's prompt is
    /// answered, the passes in a row start again, and then the damage
    /// assignment order is announced, before anyone has priority.
    fn the_blocks_are_judged_then_the_damage_assignment_order_announced() {
        // A combat of two attackers, as declared: what the blocks do to it
        // is the judging's and the recording's proofs'; over attackers
        // attacking or not, this read the blocks back out of a combat of a
        // size the solver left open, and ran out of memory.
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let mut combat = CombatState::default();
        for _ in 0..2 {
            let id = state.create_object(blank, state.active_player, Zone::Battlefield, Some(1), Some(1));
            combat.attackers.insert(id, state.opponent(state.active_player));
            combat.blocker_assignments.insert(id, Vec::new());
        }
        state.combat = Some(combat);
        state.awaiting_action = Some(crate::state::AwaitingAction::DeclareBlockers { defending_player: PlayerId(0) });
        state.consecutive_passes = 1;
        let submitted = [(ObjectId(3), ObjectId(1)), (ObjectId(4), ObjectId(2))];

        let applied = crate::engine::actions::combat::declare_blockers(&mut state, &submitted, &registry);

        assert!(matches!(applied, crate::engine::Applied::Continue));
        // SAFETY: as in `can_block_attacker`.
        let (steps, n, handed) = unsafe { (*std::ptr::addr_of!(STEPS.value), *std::ptr::addr_of!(STEPS_N.value), *std::ptr::addr_of!(HANDED.value)) };
        assert!(n == 2 && steps == [1, 2] && handed);
        assert!(state.awaiting_action.is_none() && state.consecutive_passes == 0);
        std::mem::forget((registry, state));
    }
}

#[allow(dead_code)] // named only by `kani::stub`
fn no_power(_: &GameState, _: ObjectId, _: &CardRegistry) -> Option<i32> {
    None
}

/// The damage assignment order announced (`combat::
/// announce_damage_assignment_order`, CR 509.2, 510.1c): attacker by
/// attacker in the board's order, each blocked attacker with one blocker
/// left to place in its order has that one placed last, without asking;
/// the first with two or more left has the attacking player asked to put
/// exactly those in order, as they block it, the question naming the
/// attacker, and no attacker after it is placed until that is answered;
/// with none, nothing is asked. For the active player either player, and
/// the blocks and the order already placed constants of each harness:
/// `first` and `second` the blockers (of 3, 4 and 5) of the attackers 1
/// and 2, in the order they block, and `placed` those of the first's
/// already placed.
fn announcing(first: &[u64], second: &[u64], placed: &[u64]) {
    // Each list in an array of room (`cards::in_room`): an empty one, as
    // an array of no length, ran every one of these out of 12 GB.
    let (f, s, p): ([u64; 3], [u64; 3], [u64; 3]) = (in_room(first, 0), in_room(second, 0), in_room(placed, 0));
    let (first, second, placed) = (&f[..first.len()], &s[..second.len()], &p[..placed.len()]);
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let active = PlayerId(u8::from(kani::any::<bool>()));
    state.active_player = active;
    let defending = state.opponent(active);
    let mut combat = CombatState::default();
    let ids = |list: &[u64]| -> Vec<ObjectId> { list.iter().map(|&i| ObjectId(i)).collect() };
    // Each attacker by itself, not in a loop over `[first, second]`: the
    // slices read back out of the array iterator's storage were ones the
    // solver could not follow, and every object made after went in as one
    // that might replace another (every one of these ran out of 12 GB; the
    // same board made so passes in 29 s).
    let attacker = |state: &mut GameState, combat: &mut CombatState, blockers: &[u64]| {
        let id = state.create_object(blank, active, Zone::Battlefield, Some(1), Some(1));
        combat.attackers.insert(id, defending);
        if !blockers.is_empty() {
            combat.blocked_attackers.insert(id);
        }
        combat.blocker_assignments.insert(id, ids(blockers));
    };
    attacker(&mut state, &mut combat, first);
    attacker(&mut state, &mut combat, second);
    for _ in 0..3 {
        state.create_object(blank, defending, Zone::Battlefield, Some(1), Some(1));
    }
    if !placed.is_empty() {
        let mut order = Vec::with_capacity(3);
        order.extend(placed.iter().map(|&i| ObjectId(i)));
        combat.damage_assignment_order.insert(ObjectId(1), order);
    }
    state.combat = Some(combat);

    crate::combat::announce_damage_assignment_order(&mut state, &registry);

    let left = |blockers: &[u64], placed: &[u64]| -> usize { blockers.iter().filter(|b| !placed.contains(b)).count() };
    let combat = state.combat.as_ref().expect("a combat");
    let none = [ObjectId(0); 1];
    let order = |a: u64| combat.damage_assignment_order.get(&ObjectId(a)).map_or(&none[..0], Vec::as_slice);
    // The first attacker: asked about, or its one blocker left placed last.
    let asked_first = left(first, placed) >= 2;
    let mut first_order: Vec<ObjectId> = Vec::with_capacity(3);
    first_order.extend(ids(placed));
    if left(first, placed) == 1 {
        first_order.extend(first.iter().filter(|b| !placed.contains(b)).map(|&i| ObjectId(i)));
    }
    let asked_second = !asked_first && left(second, &p[..0]) >= 2;
    let second_order: Vec<ObjectId> = if !asked_first && second.len() == 1 { ids(second) } else { Vec::new() };
    assert!(order(1) == first_order.as_slice() && order(2) == second_order.as_slice());
    let asked = |a: u64, expected: Vec<ObjectId>| matches!(&state.awaiting_action,
        Some(crate::state::AwaitingAction::ResolutionChoice {
            player, source,
            choice: crate::state::ResolutionChoiceKind::ChooseDamageAssignmentOrder { attacker, remaining, .. },
        }) if *player == active && *source == ObjectId(a) && *attacker == ObjectId(a) && *remaining == expected);
    if asked_first {
        let rest: Vec<ObjectId> = first.iter().filter(|b| !placed.contains(b)).map(|&i| ObjectId(i)).collect();
        assert!(asked(1, rest));
    } else if asked_second {
        assert!(asked(2, ids(second)));
    } else {
        assert!(state.awaiting_action.is_none());
    }
    std::mem::forget((registry, state, first_order, second_order));
}

/// The proofs of announcing the damage assignment order.
macro_rules! each_order {
    ($($name:ident: $first:expr, $second:expr, $placed:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::effective_power, no_power)]
            #[kani::stub(crate::state::GameState::effective_toughness, no_power)]
            /// [`announcing`] for the blocks of its name.
            fn $name() {
                announcing(&$first, &$second, &$placed);
            }
        }
    )*};
}

each_order! {
    no_order_is_asked_of_attackers_unblocked: [], [], [];
    an_attacker_blocked_by_one_has_it_placed_without_asking: [3], [4], [];
    an_attacker_blocked_by_two_has_its_player_asked_their_order: [3, 4], [], [];
    the_first_attacker_blocked_by_two_is_asked_before_any_after_it_is_placed: [4, 3], [5], [];
    an_attacker_unblocked_is_passed_over_for_the_next_blocked_by_two: [], [5, 3], [];
    the_last_blocker_left_to_place_is_placed_without_asking: [3, 4], [5], [4];
}

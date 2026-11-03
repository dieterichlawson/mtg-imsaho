//! The answers to combat's two questions (`actions::choices::resolve_choice`):
//! the damage assignment order of an attacker blocked by more than one
//! creature (CR 509.2), and how much of an attacker's damage one of its
//! blockers is assigned (510.1c-d). Which question is asked next, after an
//! answer, is `combat::announce_damage_assignment_order`'s and
//! `combat::combat_damage_step`'s (`declaring_blocks`, `combat_damage`),
//! each counted here.

use crate::actions::ResolvedChoice;
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, Blank};
use crate::state::{AwaitingAction, CombatState, GameState, ResolutionChoiceKind};
use crate::types::Zone;

/// How many times the next question was asked for: 0 the next damage
/// assignment order, 1 the step's damage (and whether first strike only).
static mut ASKED_NEXT: [usize; 2] = [0; 2];
static mut FIRST_STRIKE: Option<bool> = None;

#[allow(dead_code)] // each named only by `kani::stub`
fn next_order(_: &mut GameState, _: &CardRegistry) {
    // SAFETY: a proof runs on one thread, and only these functions write ASKED_NEXT and FIRST_STRIKE.
    unsafe { (*std::ptr::addr_of_mut!(ASKED_NEXT))[0] += 1 };
}
#[allow(dead_code)]
fn next_damage(_: &mut GameState, _: &CardRegistry, first_strike_only: bool) {
    // SAFETY: as in `next_order`.
    unsafe {
        (*std::ptr::addr_of_mut!(ASKED_NEXT))[1] += 1;
        *std::ptr::addr_of_mut!(FIRST_STRIKE) = Some(first_strike_only);
    }
}

/// An attacker of the first player's blocked by two of the second's, in
/// combat, none of its blockers placed in its damage assignment order yet.
fn blocked_by_two() -> (CardRegistry, GameState, ObjectId, [ObjectId; 2]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let attacker = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    let blockers = [
        state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(1), Some(1)),
        state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(1), Some(1)),
    ];
    let mut combat = CombatState::default();
    combat.attackers.insert(attacker, PlayerId(1));
    combat.blocker_assignments.insert(attacker, blockers.to_vec());
    combat.blocked_attackers.insert(attacker);
    state.combat = Some(combat);
    (registry, state, attacker, blockers)
}

/// The order recorded for `attacker`: its length and its first two.
fn placed(state: &GameState, attacker: ObjectId) -> (usize, [Option<ObjectId>; 2]) {
    let order = state.combat.as_ref().and_then(|c| c.damage_assignment_order.get(&attacker));
    order.map_or((0, [None, None]), |o| (o.len(), [o.first().copied(), o.get(1).copied()]))
}

/// The damage assignment order asked of an attacker blocked by two,
/// answered (CR 509.2): with the whole order, each blocker still to be
/// placed exactly once, they are placed in it in that order, and the next
/// attacker whose order is open is asked about (or none, and the step goes
/// on); with one of them, it takes the next place, and the order is asked
/// again for the rest; anything else (an order naming one twice or leaving
/// one out, or one not offered) is refused, the question standing as it
/// was asked and nothing placed. The reply each harness's: 0 and 1 one
/// blocker, 2 one past those offered, 3 and 4 the whole order either way
/// round, 5 one twice, 6 one left out.
fn ordered(reply: u8) {
    let (registry, mut state, attacker, blockers) = blocked_by_two();
    let options = vec![String::new(), String::new()];
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(0),
        source: attacker,
        choice: ResolutionChoiceKind::ChooseDamageAssignmentOrder {
            description: String::new(),
            attacker,
            remaining: blockers.to_vec(),
            options,
        },
    });
    let answer = match reply {
        0..=2 => ResolvedChoice::ChosenIndex(usize::from(reply), String::new()),
        3 => ResolvedChoice::ChosenOrder(vec![0, 1]),
        4 => ResolvedChoice::ChosenOrder(vec![1, 0]),
        5 => ResolvedChoice::ChosenOrder(vec![0, 0]),
        _ => ResolvedChoice::ChosenOrder(vec![1]),
    };

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &answer, &registry);

    // SAFETY: as in `next_order`.
    let asked = unsafe { *std::ptr::addr_of!(ASKED_NEXT) };
    let (n, first) = placed(&state, attacker);
    match reply {
        0 | 1 => {
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(n == 1 && first[0] == Some(blockers[usize::from(reply)]) && asked == [1, 0]);
        }
        3 | 4 => {
            let (a, b) = if reply == 3 { (blockers[0], blockers[1]) } else { (blockers[1], blockers[0]) };
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(n == 2 && first == [Some(a), Some(b)] && asked == [1, 0]);
        }
        _ => {
            assert!(matches!(applied, crate::engine::Applied::ReturnNow));
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player: PlayerId(0), source: s,
                choice: ResolutionChoiceKind::ChooseDamageAssignmentOrder { attacker: a, remaining, .. },
            }) if *s == attacker && *a == attacker && remaining.as_slice() == blockers));
            assert!(n == 0 && asked == [0, 0]);
        }
    }
    std::mem::forget((registry, state, answer));
}

/// The proofs of a damage assignment order answered, one per reply.
macro_rules! each_order_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::combat::announce_damage_assignment_order, next_order)]
            /// [`ordered`] for the reply of its name.
            fn $name() {
                ordered($reply);
            }
        }
    )*};
}

each_order_reply! {
    the_first_blocker_chosen_takes_the_first_place: 0;
    the_second_blocker_chosen_takes_the_first_place: 1;
    a_blocker_past_those_offered_is_refused: 2;
    the_whole_order_is_placed_as_given: 3;
    the_whole_order_reversed_is_placed_as_given: 4;
    an_order_naming_a_blocker_twice_is_refused: 5;
    an_order_leaving_a_blocker_out_is_refused: 6;
}

/// The damage one blocker is assigned, answered (CR 510.1c-d): the
/// amounts offered are those from the least the rules allow up, and the
/// one chosen is recorded as the attacker's damage to that blocker, and
/// the step goes on with its damage (asking about the next division open,
/// or dealing it all, first strike's or the rest's as the question was);
/// an amount past those offered is refused, the question standing as it
/// was asked and nothing recorded. For a least of one, three amounts
/// offered, a question in either half of the step, and the reply each
/// harness's: 0 and 2 an amount offered, 3 one past them.
fn assigned(reply: u8) {
    let (registry, mut state, attacker, blockers) = blocked_by_two();
    let first_strike_only: bool = kani::any();
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(0),
        source: attacker,
        choice: ResolutionChoiceKind::AssignCombatDamage {
            description: String::new(),
            attacker,
            blocker: blockers[0],
            min: 1,
            max: 3,
            options: vec![String::new(), String::new(), String::new()],
            first_strike_only,
        },
    });

    let applied = crate::engine::actions::choices::resolve_choice(
        &mut state, &ResolvedChoice::ChosenIndex(usize::from(reply), String::new()), &registry);

    // SAFETY: as in `next_order`.
    let (asked, strike) = unsafe { (*std::ptr::addr_of!(ASKED_NEXT), *std::ptr::addr_of!(FIRST_STRIKE)) };
    let chosen = state.combat.as_ref().and_then(|c| c.chosen_damage.get(&attacker));
    if reply < 3 {
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(matches!(chosen.map(Vec::as_slice), Some([(b, amount)]) if *b == blockers[0] && *amount == 1 + u32::from(reply)));
        assert!(asked == [0, 1] && strike == Some(first_strike_only));
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player: PlayerId(0), source: s,
            choice: ResolutionChoiceKind::AssignCombatDamage { attacker: a, blocker: b, min: 1, first_strike_only: f, .. },
        }) if *s == attacker && *a == attacker && *b == blockers[0] && *f == first_strike_only));
        assert!(chosen.is_none_or(Vec::is_empty) && asked == [0, 0]);
    }
    std::mem::forget((registry, state));
}

/// The proofs of a division answered, one per reply.
macro_rules! each_division_reply {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::combat::combat_damage_step, next_damage)]
            /// [`assigned`] for the reply of its name.
            fn $name() {
                assigned($reply);
            }
        }
    )*};
}

each_division_reply! {
    the_least_damage_offered_is_assigned: 0;
    the_most_damage_offered_is_assigned: 2;
    damage_past_that_offered_is_refused: 3;
}

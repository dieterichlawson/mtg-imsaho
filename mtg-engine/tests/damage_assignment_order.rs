//! CR 509.2 / 510.1c: the attacking player announces the damage assignment
//! order among an attacker's blockers, and damage is assigned in it.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use mtg_engine::types::*;

/// The blockers still on offer at a pending damage-assignment-order prompt,
/// or `None` if no such prompt is up.
fn pending_order_prompt(state: &GameState) -> Option<(mtg_engine::ids::PlayerId, Vec<ObjectId>, Vec<String>)> {
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            player,
            choice: ResolutionChoiceKind::ChooseDamageAssignmentOrder { remaining, options, .. },
            ..
        }) => Some((*player, remaining.clone(), options.clone())),
        _ => None,
    }
}

/// Answer the pending order prompt by naming `blocker`.
fn order_next(state: &mut GameState, blocker: ObjectId, registry: &CardRegistry) {
    let (_, remaining, options) = pending_order_prompt(state)
        .expect("expected a damage assignment order prompt");
    let index = remaining.iter().position(|&b| b == blocker)
        .expect("blocker is not among the ones still to be ordered");
    *state = mtg_engine::engine::submit_action(
        state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenIndex(index, options[index].clone()),
        },
        registry,
    );
}

/// CR 509.2: two blockers on one attacker is a choice, and it belongs to the
/// attacking player — not to the defender who declared the blocks.
#[test]
fn a_double_block_asks_the_attacking_player_for_the_order() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let small = ready_creature(&mut state, P1, 1, 1);
    let big = ready_creature(&mut state, P1, 3, 3);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(small, attacker), (big, attacker)], &reg);

    let (player, remaining, options) = pending_order_prompt(&state)
        .expect("a double block must raise the CR 509.2 announcement");
    assert_eq!(player, P0, "the attacking player announces the order (CR 509.2)");
    assert_eq!(remaining.len(), 2);
    assert_eq!(options.len(), 2);
}

/// One blocker admits one order. Nothing is asked, and the order is recorded
/// anyway so the damage step has it.
#[test]
fn a_single_blocker_is_not_worth_a_prompt() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);

    assert!(pending_order_prompt(&state).is_none(), "one blocker is not a choice");
    assert_eq!(
        state.combat.as_ref().unwrap().damage_assignment_order.get(&attacker),
        Some(&vec![blocker]),
        "the forced order is still recorded"
    );
}

/// An unblocked attacker has nothing to order.
#[test]
fn an_unblocked_attacker_raises_no_prompt() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[], &reg);

    assert!(pending_order_prompt(&state).is_none());
}

/// The whole point of the choice: a 3/3 attacker double-blocked by a 1/1 and
/// a 3/3 kills whichever blocker the attacking player puts first, and the
/// engine no longer decides that for them (CR 510.1c).
#[test]
fn the_announced_order_decides_which_blocker_dies() {
    for order_big_first in [false, true] {
        let reg = registry();
        let mut state = game_at_step(Step::CombatDamage, P0);
        let attacker = ready_creature(&mut state, P0, 3, 3);
        let small = ready_creature(&mut state, P1, 1, 1);
        let big = ready_creature(&mut state, P1, 3, 3);

        submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
        // Declaration order is small-then-big in both runs, so anything that
        // changes below is the announcement and not the declaration.
        submit_declare_blockers(&mut state, P1, &[(small, attacker), (big, attacker)], &reg);

        if order_big_first {
            order_next(&mut state, big, &reg);
        } else {
            order_next(&mut state, small, &reg);
        }
        assert!(pending_order_prompt(&state).is_none(), "the last place is forced");

        combat::deal_combat_damage(&mut state, &reg);

        let small_damage = state.get_object(small).unwrap().damage_marked;
        let big_damage = state.get_object(big).unwrap().damage_marked;
        if order_big_first {
            assert_eq!(big_damage, 3, "all three went to the 3/3 named first");
            assert_eq!(small_damage, 0, "nothing was left for the 1/1");
        } else {
            assert_eq!(small_damage, 1, "lethal to the 1/1 named first (CR 510.1c)");
            assert_eq!(big_damage, 2, "the excess went to the 3/3, which survives");
        }
    }
}

/// The order is announced once and stands for both combat damage steps
/// (CR 510.4): a double striker assigns its second damage in the same order,
/// not in a re-derived one.
#[test]
fn both_damage_steps_use_the_one_announced_order() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::DoubleStrike);
    let first = ready_creature(&mut state, P1, 2, 2);
    let second = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(first, attacker), (second, attacker)], &reg);
    // Announce the declaration order reversed, so "the announced order" and
    // "the order the blocks arrived in" cannot be confused.
    order_next(&mut state, second, &reg);

    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(second).unwrap().damage_marked, 2,
        "both of the double striker's damage steps assigned to the blocker named first");
    assert_eq!(state.get_object(first).unwrap().damage_marked, 0,
        "the blocker named second was never reached");
}

/// A blocker that leaves combat between the announcement and the damage step
/// is skipped without disturbing the rest of the order (CR 506.4c).
#[test]
fn a_blocker_that_leaves_combat_drops_out_of_the_order() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 4, 4);
    let gone = ready_creature(&mut state, P1, 1, 1);
    let survivor = ready_creature(&mut state, P1, 3, 3);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(gone, attacker), (survivor, attacker)], &reg);
    order_next(&mut state, gone, &reg);

    state.move_object(gone, Zone::Graveyard, &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(survivor).unwrap().damage_marked, 4,
        "with the first blocker gone, everything is assigned to the next one in order");
}

/// Issue #325: the whole damage assignment order in one answer (CR 509.2),
/// recorded and logged as the order it is.
#[test]
fn a_whole_order_places_every_blocker_at_once() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 5, 5);
    let a = ready_creature(&mut state, P1, 1, 1);
    let b = ready_creature(&mut state, P1, 2, 2);
    let c = ready_creature(&mut state, P1, 3, 3);
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(a, attacker), (b, attacker), (c, attacker)], &reg);

    let (_, remaining, _) = pending_order_prompt(&state).expect("a triple block asks for the order");
    let index_of = |x| remaining.iter().position(|&r| r == x).expect("offered");
    // c first, then a, then b.
    let order = vec![index_of(c), index_of(a), index_of(b)];
    let bad = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenOrder(vec![0, 0, 1]),
    }, &reg);
    assert!(pending_order_prompt(&bad).is_some(), "a non-permutation is refused and the prompt stands");

    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenOrder(order),
    }, &reg);
    assert!(pending_order_prompt(&state).is_none(), "one answer settles the attacker");
    assert_eq!(state.combat.as_ref().unwrap().damage_assignment_order.get(&attacker),
        Some(&vec![c, a, b]));
    assert!(state.game_log.iter().any(|e| e.message.contains("announced the damage assignment order")),
        "the order is logged: {:?}", state.game_log.iter().map(|e| &e.message).collect::<Vec<_>>());
}

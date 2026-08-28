//! CR 601.2c on the way *in*: the targets a player hands the engine.
//!
//! `legal_actions` enumerates only legal target sets, and for a long time that
//! was the whole of the enforcement — every submit path took the list it was
//! given. That is not a theoretical gap: neither client picks a whole offered
//! action. Both `mtg-player`'s CLI and its LLM driver assemble their own
//! action from per-slot choices, so the list the engine receives is one it
//! never built.
//!
//! Five card audits each met a different face of it — Corpse Lunge, Unburial
//! Rites, Purify the Grave, Travel Preparations, Rage Thrower — and each was
//! patched where it was found. These are the three submit paths, checked as
//! one rule: casting a spell, activating an ability, and answering a
//! resolution-time target prompt.
//!
//! The refusal is a no-op, not a partial action: it happens before any cost is
//! paid, because an illegal choice means the thing did not happen rather than
//! that it happened for nothing.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::types::*;

/// Casting: a spell whose targets were never legal does not go on the stack,
/// and nothing is paid for it.
///
/// Bump in the Night is "target opponent loses 3 life"; the opponent here has
/// hexproof from Witchbane Orb, so they were never an offerable target.
#[test]
fn a_cast_with_a_target_that_was_never_legal_does_not_happen() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Witchbane Orb", P1);

    let bump = castable_spell(&mut state, &reg, "Bump in the Night", P0);
    let mana_before = state.get_player(P0).mana_pool.clone();
    let their_life = state.get_player(P1).life;

    let state = cast_onto_stack(&state, &reg, bump, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(bump).unwrap().zone, Zone::Hand,
        "the spell never left the hand");
    assert!(state.stack.is_empty(), "and nothing went on the stack");
    assert_eq!(state.get_player(P0).mana_pool, mana_before,
        "and no mana was paid — an illegal choice means the cast did not \
         happen, not that it happened for nothing");
    assert_eq!(state.get_player(P1).life, their_life);
}

/// Activating: an ability whose target was never legal is refused, and the
/// activation cost is not paid.
///
/// Avacynian Priest is "{1}, {T}: Tap target non-Human creature", declared as
/// `TargetRequirement::Creature` with the non-Human half in the card's own
/// `is_valid_target`. A land fails the first half, which is the generic one.
#[test]
fn an_activation_with_a_target_that_was_never_legal_does_not_happen() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P1);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    let mana_before = state.get_player(P0).mana_pool.clone();

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: priest,
            ability_index: 0,
            targets: vec![Target::Object(land)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        },
        &reg,
    );

    assert!(!state.get_object(priest).unwrap().tapped,
        "the tap cost was not paid");
    assert_eq!(state.get_player(P0).mana_pool, mana_before,
        "and neither was the mana");
    assert!(state.stack.is_empty(), "and nothing went on the stack");
}

/// Answering a prompt: a target that was not among the options offered is not
/// an answer, and the prompt stays open.
///
/// Rage Thrower's "target player or planeswalker" is the one this was found
/// on — a creature is not either, and submitting one used to be taken at face
/// value.
#[test]
fn a_choice_answered_with_something_never_offered_is_refused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _thrower = named_permanent(&mut state, &reg, "Rage Thrower", P0);
    let bystander = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let victim = ready_creature(&mut state, P1, 1, 1);

    kill_by_damage(&mut state, &reg, victim);
    mtg_engine::triggers::process_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "test setup: the trigger asks for a target");

    let life_before = state.get_player(P1).life;
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bystander))),
        },
        &reg,
    );

    assert!(state.awaiting_action.is_some(),
        "a creature was never offered, so the prompt is still waiting");
    assert_eq!(state.get_object(bystander).unwrap().damage_marked, 0,
        "and nothing was damaged");
    assert_eq!(state.get_player(P1).life, life_before);
}

/// And an answer of the wrong *shape* is refused the same way: the question
/// asked for a target, so a yes/no does not answer it.
///
/// This used to fall through the match silently — the prompt was taken off
/// the state and dropped, which resumed a resolution that never got its
/// choice.
#[test]
fn a_choice_answered_in_the_wrong_shape_is_refused() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _thrower = named_permanent(&mut state, &reg, "Rage Thrower", P0);
    let victim = ready_creature(&mut state, P1, 1, 1);

    kill_by_damage(&mut state, &reg, victim);
    mtg_engine::triggers::process_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "test setup: the trigger asks for a target");

    let life_before = state.get_player(P1).life;
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    assert!(state.awaiting_action.is_some(),
        "the target question is still unanswered");
    assert_eq!(state.get_player(P1).life, life_before);
}

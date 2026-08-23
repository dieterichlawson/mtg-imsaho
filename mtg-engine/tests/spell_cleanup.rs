//! Regression tests for engine-owned spell cleanup (CR 608.2m).
//!
//! A spell that presents player choices during resolution must stay on the
//! stack until the choice chain completes; moving it to the graveyard is
//! the FINAL step of resolution and is owned by the engine, not card code.
//! Divine Reckoning previously moved itself to the graveyard before
//! presenting its keep-a-creature choices.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::AwaitingAction;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Answer the current ChooseTarget resolution choice with a specific object.
fn answer_choice_with(
    state: &mtg_engine::state::GameState,
    reg: &CardRegistry,
    chosen: mtg_engine::ids::ObjectId,
) -> mtg_engine::state::GameState {
    let Some(AwaitingAction::ResolutionChoice { choice, .. }) = &state.awaiting_action else {
        panic!("expected a pending resolution choice, got {:?}", state.awaiting_action);
    };
    let mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. } = choice else {
        panic!("expected ChooseTarget, got {choice:?}");
    };
    let target = Target::Object(chosen);
    assert!(options.contains(&target), "{chosen:?} not among options {options:?}");
    engine::submit_action(
        state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(target)) },
        reg,
    )
}

/// Divine Reckoning must stay on the stack while its keep-a-creature
/// choices are pending, and reach the graveyard only after the chain
/// completes.
#[test]
fn divine_reckoning_stays_on_stack_until_choices_complete() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Both players control two creatures, so both get a real choice.
    let keep0 = ready_creature(&mut state, P0, 1, 1);
    let kill0 = ready_creature(&mut state, P0, 2, 2);
    let keep1 = ready_creature(&mut state, P1, 3, 3);
    let kill1 = ready_creature(&mut state, P1, 4, 4);

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Mid-resolution: the spell must still be on the stack (CR 608.2m —
    // moving to the graveyard is the final step of resolution).
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Stack,
        "Divine Reckoning must remain on the stack while choices are pending");
    assert!(state.awaiting_action.is_some(), "first player's keep choice should be pending");

    // P0 keeps keep0; then P1 keeps keep1.
    state = answer_choice_with(&state, &reg, keep0);
    assert!(state.awaiting_action.is_some(), "second player's keep choice should be pending");
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Stack,
        "spell must still be on the stack between chained choices");
    state = answer_choice_with(&state, &reg, keep1);

    // Chain complete: engine moves the spell to the graveyard.
    assert!(state.awaiting_action.is_none());
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Graveyard,
        "engine must move the spell to the graveyard once the choice chain completes");

    // And the effect itself worked: each player kept exactly one creature.
    assert_eq!(state.get_object(keep0).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(keep1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(kill0).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(kill1).unwrap().zone, Zone::Graveyard);
}

/// A spell with no player choices is cleaned up immediately by resolve_spell,
/// and the resolving-spell tracker does not leak into later actions.
#[test]
fn immediate_resolution_cleanup_and_no_tracker_leak() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = ready_creature(&mut state, P1, 4, 4);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(target)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);
    assert!(state.resolving_spell.is_none(), "tracker must be cleared after immediate cleanup");
}

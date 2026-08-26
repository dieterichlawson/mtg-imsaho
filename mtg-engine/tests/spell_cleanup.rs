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
    state = cast_onto_stack(&state, &reg, spell, vec![]);

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
    state = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(target)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);
    assert!(state.resolving_spell.is_none(), "tracker must be cleared after immediate cleanup");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Night Terrors is never moved off the stack when the target
/// player has multiple nonland cards in hand (choice mechanism fails).
#[test]
fn bug_night_terrors_stuck_on_stack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P1 multiple nonland cards in hand
    for name in ["Grizzly Bears", "Lightning Bolt", "Giant Growth"] {
        spell_in_hand(&mut state, &registry, name, P1);
    }

    // Cast Night Terrors targeting P1
    let nt = castable_spell(&mut state, &registry, "Night Terrors", P0);
    state = cast_and_resolve(&state, &registry, nt, vec![Target::Player(P1)]);

    // Resolve any pending choices
    // The spell should either be in graveyard (resolved) or awaiting a choice
    let _nt_zone = state.get_object(nt).unwrap().zone;
    let has_choice = state.awaiting_action.is_some();

    // With multiple nonland cards, a choice should be presented
    assert!(has_choice,
        "Night Terrors should present choice for multiple nonland cards");

    // Simulate choosing the first option
    if let Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. },
        ..
    }) = &state.awaiting_action {
        if let Some(first_target) = options.first() {
            let choice_action = Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(first_target.clone())),
            };
            state = engine::submit_action(&state, &choice_action, &registry);
        }
    }

    // After resolving the choice, Night Terrors should be in the graveyard
    let nt_zone_after = state.get_object(nt).unwrap().zone;
    // BUG: Night Terrors stays on the stack because ExileAndStore doesn't
    // call move_spell_after_resolve for the source spell
    assert_eq!(nt_zone_after, Zone::Graveyard,
        "Night Terrors should be in graveyard after choice resolves. Zone: {nt_zone_after:?}");
}

/// Bug: Night Terrors uses `ExileAndStore` as its `PendingEffect`, but
/// it should just exile (not store). `ExileAndStore` is for Fiend Hunter-
/// style effects that need to track what was exiled for later return.
#[test]
fn bug_night_terrors_wrong_pending_effect() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P1 multiple nonland cards
    let _card1 = spell_in_hand(&mut state, &registry, "Grizzly Bears", P1);
    let _card2 = spell_in_hand(&mut state, &registry, "Lightning Bolt", P1);

    // Cast Night Terrors targeting P1
    let nt = castable_spell(&mut state, &registry, "Night Terrors", P0);
    state = cast_and_resolve(&state, &registry, nt, vec![Target::Player(P1)]);

    // Check what PendingEffect is used in the choice
    let uses_exile_and_store = state.awaiting_action.as_ref().is_some_and(|aa| {
        format!("{aa:?}").contains("ExileAndStore")
    });

    // BUG: Uses ExileAndStore instead of plain Exile
    assert!(!uses_exile_and_store,
        "Night Terrors should use a plain Exile effect, not ExileAndStore");
}

//! Regression tests for CR 603.2 / 603.3d trigger dispatch:
//! - Conditional SpellCast watchers (Charmbreaker Devils) must not create
//!   stack entries for spells that don't satisfy the trigger condition.
//! - ETB triggers with declared target requirements (Fiend Hunter) lock
//!   their target as the trigger goes on the stack; creatures entering
//!   afterwards are not legal targets.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind, StackEntry};
use mtg_engine::triggers;
use mtg_engine::types::*;
fn trigger_count(state: &mtg_engine::state::GameState) -> usize {
    state.stack.iter().filter(|e| matches!(e, StackEntry::Trigger(_))).count()
}

/// Charmbreaker Devils: "Whenever YOU cast an INSTANT OR SORCERY spell" —
/// an opponent's spell, or a creature spell, must not create a trigger.
#[test]
fn charmbreaker_devils_trigger_only_for_own_instants_and_sorceries() {
    let reg = registry();

    // Case 1: opponent casts an instant — no trigger for P0's Devils.
    let mut state = game_at_step(Step::PrecombatMain, P1);
    let _devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P1);
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P0)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 0,
        "opponent's spell must not put Charmbreaker Devils' trigger on the stack (CR 603.2)");

    // Case 2: controller casts a creature spell — still no trigger.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 0,
        "a creature spell must not put Charmbreaker Devils' trigger on the stack (CR 603.2)");

    // Case 3: controller casts an instant — the trigger IS created.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 1,
        "own instant should put exactly one Charmbreaker Devils trigger on the stack");
}

/// Fiend Hunter's ETB target is locked when the trigger goes on the stack:
/// a creature that enters afterwards must not be offered at resolution.
#[test]
fn fiend_hunter_target_locked_at_trigger_creation() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Exactly one other creature — the push auto-locks it as the target.
    let victim = ready_creature(&mut state, P1, 2, 2);

    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 1, "ETB trigger should be on the stack with its target locked");

    // A creature enters AFTER the trigger was put on the stack.
    let latecomer = ready_creature(&mut state, P1, 5, 5);

    // Resolve the trigger: the "you may" prompt must offer ONLY the locked
    // target, not the latecomer.
    triggers::process_triggers(&mut state, &reg);
    let Some(AwaitingAction::ResolutionChoice { choice, .. }) = &state.awaiting_action else {
        panic!("expected the optional exile choice, got {:?}", state.awaiting_action);
    };
    let ResolutionChoiceKind::ChooseTarget { options, optional, .. } = choice else {
        panic!("expected ChooseTarget, got {choice:?}");
    };
    assert!(*optional, "'you may' — declining must be possible");
    assert_eq!(options, &vec![Target::Object(victim)],
        "resolution must offer only the target locked at trigger creation (CR 603.3d)");

    // Accept: the locked target is exiled, the latecomer untouched.
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(victim))) },
        &reg,
    );
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(latecomer).unwrap().zone, Zone::Battlefield);
}

/// If the locked target is gone at resolution, the trigger is countered by
/// game rules (CR 608.2b) and Fiend Hunter exiles nothing.
#[test]
fn fiend_hunter_trigger_fizzles_when_locked_target_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = ready_creature(&mut state, P1, 2, 2);
    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 1);

    // The locked target dies in response.
    state.move_object(victim, Zone::Graveyard, &reg);

    triggers::process_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_none(),
        "fizzled trigger must not present the exile choice");
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard,
        "the dead creature stays in the graveyard");
}

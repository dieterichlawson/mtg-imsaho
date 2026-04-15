//! Tests for phantom trigger bugs:
//! 1. Creatures without ETB abilities should not create self-ETB triggers.
//! 2. ETB-watch triggers should only fire from zones declared by `trigger_zones()`.
//!    E.g., Champion of the Parish in the graveyard should NOT trigger.

use mtg_engine::cards::CardRegistry;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers;
use mtg_engine::types::*;

mod common;
use common::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ---------------------------------------------------------------------------
// Bug 1: Self-ETB phantom triggers
// ---------------------------------------------------------------------------

/// A creature with no ETB ability (Vampire Interloper) should not create
/// any trigger on the stack when it enters the battlefield.
#[test]
fn no_phantom_self_etb_for_creature_without_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Vampire Interloper directly on the battlefield and emit the ETB event.
    let interloper = named_creature(&mut state, &reg, "Vampire Interloper", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: interloper,
        controller: P0,
    });

    let had_triggers = triggers::collect_triggers(&mut state, &reg);
    assert!(!had_triggers, "No triggers should be collected for a creature without ETB");
    assert!(
        state.stack.is_empty(),
        "Stack should be empty — Vampire Interloper has no ETB ability"
    );
}

/// A creature WITH an ETB ability still correctly creates a trigger.
#[test]
fn real_etb_trigger_still_fires() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let crossway = named_creature(&mut state, &reg, "Crossway Vampire", P0);
    // Need a valid target for the ETB.
    let _enemy = named_creature(&mut state, &reg, "Vampire Interloper", P1);

    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: crossway,
        controller: P0,
    });

    let had_triggers = triggers::collect_triggers(&mut state, &reg);
    assert!(had_triggers, "Crossway Vampire has an ETB trigger");
    // CR 603.3d: with two valid creature targets (Crossway itself and
    // Vampire Interloper), the trigger pauses for a target-choice prompt
    // before going on the stack.
    assert!(
        state.awaiting_action.is_some()
            || state.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_))),
        "ETB trigger should be on the stack or awaiting a target choice"
    );
}

// ---------------------------------------------------------------------------
// Bug 2: ETB-watch triggers from the wrong zone
// ---------------------------------------------------------------------------

/// Champion of the Parish in the graveyard should NOT create a watch trigger
/// when a creature enters the battlefield.
#[test]
fn champion_in_graveyard_does_not_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Champion of the Parish in the graveyard.
    let card_id = reg.get_id_by_name("Champion of the Parish").unwrap();
    let data = reg.card_data(card_id).unwrap();
    let champion = state.create_object(card_id, P0, Zone::Graveyard, data.power, data.toughness);
    state.get_object_mut(champion).unwrap().name = "Champion of the Parish".into();

    // A Human enters the battlefield under the same controller.
    let human = named_creature(&mut state, &reg, "Unruly Mob", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P0,
    });

    let had_triggers = triggers::collect_triggers(&mut state, &reg);
    assert!(
        !had_triggers,
        "Champion in graveyard should NOT create a watch trigger"
    );
    assert!(
        state.stack.is_empty(),
        "No triggers should be on the stack"
    );
}

/// Champion of the Parish on the battlefield DOES trigger when a Human enters.
#[test]
fn champion_on_battlefield_does_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _champion = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    let human = named_creature(&mut state, &reg, "Unruly Mob", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P0,
    });

    let had_triggers = triggers::collect_triggers(&mut state, &reg);
    assert!(had_triggers, "Champion on battlefield should trigger for Human ETB");
    assert!(
        state.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_))),
        "Watch trigger should be on the stack"
    );
}

/// Dearly Departed in the graveyard adds a +1/+1 counter to entering
/// Humans via a CR 614.1c replacement effect (not a trigger).
#[test]
fn dearly_departed_in_graveyard_adds_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Dearly Departed in the graveyard.
    let card_id = reg.get_id_by_name("Dearly Departed").unwrap();
    let data = reg.card_data(card_id).unwrap();
    let _departed = state.create_object(card_id, P0, Zone::Graveyard, data.power, data.toughness);

    // A Human enters via move_object so the replacement effect fires.
    let human_id = reg.get_id_by_name("Unruly Mob").unwrap();
    let human = state.create_object(human_id, P0, Zone::Hand, Some(1), Some(1));
    state.get_object_mut(human).unwrap().name = "Unruly Mob".into();
    state.move_object(human, Zone::Battlefield, &reg);

    assert_eq!(counters_of(&state, human, mtg_engine::types::CounterType::PlusOnePlusOne), 1,
        "Human entering with Dearly Departed in graveyard should get +1/+1 counter");
}

/// Dearly Departed on the battlefield should NOT trigger (it only fires from graveyard).
#[test]
fn dearly_departed_on_battlefield_does_not_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _departed = named_creature(&mut state, &reg, "Dearly Departed", P0);

    let human = named_creature(&mut state, &reg, "Unruly Mob", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P0,
    });

    let had_triggers = triggers::collect_triggers(&mut state, &reg);
    assert!(
        !had_triggers,
        "Dearly Departed on battlefield should NOT create a watch trigger"
    );
}

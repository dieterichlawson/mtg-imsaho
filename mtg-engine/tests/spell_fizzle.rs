//! Tests for spell fizzle behavior (rule 608.2b/c).
//!
//! When a spell's targets all become illegal before resolution, it should
//! be countered by game rules ("fizzle"). These tests document the expected
//! behavior and verify what currently happens.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// When a targeted creature dies before a spell resolves, the spell
/// should fizzle. Currently the spell silently does nothing and goes
/// to the graveyard — this test documents that behavior.
///
/// Scenario: P0 casts Lightning Bolt targeting P1's 1/1.
/// P1 responds with Swords to Plowshares exiling the 1/1.
/// Lightning Bolt resolves but its target is gone.
#[test]
fn bolt_target_dies_before_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a 1/1 creature.
    let creature = ready_creature(&mut state, P1, 1, 1);

    // P0 casts Lightning Bolt targeting the creature.
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(creature)] },
        &reg,
    );
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Stack);

    // Before the bolt resolves, the creature is removed (e.g., exiled by another spell).
    state.move_object(creature, Zone::Graveyard);

    // Resolve the bolt — target is no longer on the battlefield.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Bolt should be in graveyard (it resolved, even though target was gone).
    assert_eq!(
        state.get_object(bolt).unwrap().zone,
        Zone::Graveyard,
        "Lightning Bolt should go to graveyard after resolving"
    );
    // Player life unchanged — bolt didn't redirect to player.
    assert_eq!(state.get_player(P1).life, 20);
}

/// A pump spell targeting a creature that has left the battlefield
/// should have no effect.
#[test]
fn giant_growth_target_dies_before_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    let growth = castable_spell(&mut state, &reg, "Giant Growth", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: growth, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Creature dies before Giant Growth resolves.
    state.move_object(creature, Zone::Graveyard);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Creature should still be in graveyard, not buffed on battlefield.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        state.get_object(growth).unwrap().zone,
        Zone::Graveyard,
        "Giant Growth should go to graveyard after resolving with no legal target"
    );
}

/// A destruction spell targeting a creature that has already left
/// the battlefield should have no effect.
#[test]
fn doom_blade_target_already_gone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let doom = castable_spell(&mut state, &reg, "Doom Blade", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: doom, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Creature is sacrificed before Doom Blade resolves.
    state.move_object(creature, Zone::Graveyard);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(doom).unwrap().zone, Zone::Graveyard);
    // The creature was already in graveyard — nothing else should happen.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Counterspell targets a spell on the stack. If that spell is somehow
/// removed from the stack before Counterspell resolves, it fizzles.
#[test]
fn counterspell_target_removed_from_stack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts a Lightning Bolt targeting P1.
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)] },
        &reg,
    );

    // P1 casts Counterspell targeting the Bolt.
    state.priority_player = Some(P1);
    let counter = castable_spell(&mut state, &reg, "Counterspell", P1);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: counter, targets: vec![Target::Object(bolt)] },
        &reg,
    );

    // Bolt is somehow removed from the stack before Counterspell resolves
    // (e.g., another Counterspell countered it first).
    state.stack.retain(|&id| id != bolt);
    state.move_object(bolt, Zone::Graveyard);

    // Resolve the Counterspell — its target (bolt) is no longer on the stack.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Counterspell should go to graveyard (resolved or fizzled).
    assert_eq!(state.get_object(counter).unwrap().zone, Zone::Graveyard);
    // P1 life unchanged — bolt never resolved (was removed).
    assert_eq!(state.get_player(P1).life, 20);
}

/// Aura spell whose target dies before resolution should go to graveyard
/// (not the battlefield).
#[test]
fn aura_target_dies_before_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);

    let pacifism = castable_spell(&mut state, &reg, "Pacifism", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: pacifism, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Creature dies before Pacifism resolves.
    state.move_object(creature, Zone::Graveyard);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Pacifism should NOT be on the battlefield (no legal target to enchant).
    assert_eq!(
        state.get_object(pacifism).unwrap().zone,
        Zone::Graveyard,
        "Aura with no legal target on resolution should go to graveyard"
    );
}

//! Tests for spell fizzle behavior (rule 608.2b/c).
//!
//! When a spell's targets all become illegal before resolution, it should
//! be countered by game rules ("fizzle"). These tests document the expected
//! behavior and verify what currently happens.
//!
//! Also tests multi-target spell behavior: partial target illegality should
//! still allow the spell to resolve for remaining legal targets (608.2c).

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
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
    state.stack.retain(|e| e.as_spell() != Some(bolt));
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

// ── Hexproof gained before resolution ──────────────────────────────

/// If a creature gains hexproof after a spell is cast targeting it,
/// the spell should still resolve (hexproof is only checked on cast,
/// not on resolution in the current engine). This documents the engine's
/// current behavior.
#[test]
fn bolt_target_gains_hexproof_before_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    // P0 casts Lightning Bolt targeting P1's creature.
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Creature gains hexproof before bolt resolves.
    state.until_end_of_turn_keywords.push(
        mtg_engine::state::UntilEndOfTurnKeyword {
            target: creature,
            keyword: Keyword::Hexproof,
        },
    );

    // Resolve the bolt — note that per MTG rules, the spell should fizzle
    // because hexproof makes the target illegal. The current engine does
    // NOT check target legality on resolution, so the bolt still deals damage.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Current behavior: bolt still deals damage despite hexproof.
    // When fizzle is implemented, this should change to:
    //   assert_eq!(state.get_object(creature).unwrap().damage_marked, 0);
    assert_eq!(
        state.get_object(creature).unwrap().damage_marked, 3,
        "Current engine behavior: bolt resolves despite hexproof gained after cast"
    );
}

// ── Multi-target spells ────────────────────────────────────────────

/// A two-target spell (Feeling of Dread: tap up to 2 creatures) should
/// still tap the surviving target even if one target dies.
#[test]
fn multi_target_spell_with_one_target_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature_a = ready_creature(&mut state, P1, 3, 3);
    let creature_b = ready_creature(&mut state, P1, 2, 2);

    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: dread,
            targets: vec![Target::Object(creature_a), Target::Object(creature_b)],
        },
        &reg,
    );

    // Creature A dies before resolution.
    state.move_object(creature_a, Zone::Graveyard);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Creature B should still be tapped (partial resolution).
    assert!(
        state.get_object(creature_b).unwrap().tapped,
        "Feeling of Dread should still tap creature B even if creature A died (rule 608.2b)"
    );
}

/// A two-target spell where BOTH targets die should have no effect.
#[test]
fn multi_target_spell_with_all_targets_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature_a = ready_creature(&mut state, P1, 3, 3);
    let creature_b = ready_creature(&mut state, P1, 2, 2);

    let dread = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: dread,
            targets: vec![Target::Object(creature_a), Target::Object(creature_b)],
        },
        &reg,
    );

    // Both creatures die before resolution.
    state.move_object(creature_a, Zone::Graveyard);
    state.move_object(creature_b, Zone::Graveyard);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // With all targets illegal, the spell should fizzle (no effect).
    // Feeling of Dread just goes to graveyard.
    assert_eq!(
        state.get_object(dread).unwrap().zone,
        Zone::Graveyard,
        "Spell with all targets dead should go to graveyard"
    );
}

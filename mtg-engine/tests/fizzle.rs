//! Tests for proper spell fizzle implementation (CR 608.2b).
//!
//! A spell whose targets are all illegal when it tries to resolve should
//! be countered by game rules BEFORE on_resolve is called. This is different
//! from on_resolve checking targets and doing nothing — the spell should
//! never resolve at all.
//!
//! These tests verify the mechanism, not just the outcome.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::events::GameEvent;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ════════════════════════════════════════════════════════════════════
// Mechanism test: SpellResolved event should NOT be emitted for fizzle
// ════════════════════════════════════════════════════════════════════

/// When a spell fizzles, the engine should NOT emit SpellResolved.
/// A fizzled spell was countered by game rules — it did not resolve.
#[test]
fn fizzled_spell_does_not_emit_resolved_event() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Target dies before resolution.
    state.move_object(creature, Zone::Graveyard);
    state.events.clear();

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should NOT have a SpellResolved event — the spell fizzled.
    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { object } if *object == bolt)
    });
    assert!(!has_resolved,
        "Fizzled spell should NOT emit SpellResolved — it was countered by game rules, not resolved (CR 608.2b)");
}

/// A spell that resolves normally SHOULD emit SpellResolved.
#[test]
fn resolved_spell_emits_resolved_event() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(creature)] },
        &reg,
    );
    state.events.clear();

    // Target is still on the battlefield — spell resolves normally.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { object } if *object == bolt)
    });
    assert!(has_resolved,
        "Spell with legal target should emit SpellResolved");
}

// ════════════════════════════════════════════════════════════════════
// Mechanism test: on_resolve should NOT be called for fizzle
//
// We test this using Swords to Plowshares, which exiles a creature
// AND causes its controller to gain life. If on_resolve runs but the
// zone check saves us, the life gain won't happen — but that's the
// wrong reason. We verify via the SpellResolved event instead.
//
// More importantly: if someone later writes a card that does
// "deal damage to target creature; draw a card", the fizzle must
// prevent BOTH effects. We simulate this with a two-part test.
// ════════════════════════════════════════════════════════════════════

/// Swords to Plowshares fizzle: no exile, no life gain, no SpellResolved.
#[test]
fn swords_fizzle_no_side_effects() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 5, 5);
    let swords = castable_spell(&mut state, &reg, "Swords to Plowshares", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: swords, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Target leaves battlefield before resolution.
    state.move_object(creature, Zone::Exile);
    state.events.clear();

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // No SpellResolved — fizzled.
    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { .. })
    });
    assert!(!has_resolved,
        "Swords to Plowshares should fizzle (not resolve) when target is gone");

    // P1 should NOT have gained life (the creature was 5/5).
    assert_eq!(state.get_player(P1).life, 20,
        "No life gain from fizzled Swords to Plowshares");
}

// ════════════════════════════════════════════════════════════════════
// Flashback fizzle: should go to exile, not graveyard
// ════════════════════════════════════════════════════════════════════

/// A flashback spell that fizzles should still be exiled (not go to graveyard).
/// The "exile instead of graveyard" replacement applies regardless of whether
/// the spell resolved or fizzled.
#[test]
fn flashback_spell_fizzle_goes_to_exile() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Geistflame in graveyard (for flashback).
    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let geistflame = state.create_object(gf_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(geistflame).unwrap().name = "Geistflame".into();

    // Add flashback mana ({3}{R}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let creature = ready_creature(&mut state, P1, 2, 2);

    // Cast with flashback.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: geistflame, targets: vec![Target::Object(creature)] },
        &reg,
    );
    assert!(state.get_object(geistflame).unwrap().cast_with_flashback);

    // Target dies before resolution.
    state.move_object(creature, Zone::Graveyard);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Flashback spell should be in exile (not graveyard), even though it fizzled.
    assert_eq!(state.get_object(geistflame).unwrap().zone, Zone::Exile,
        "Flashback spell that fizzles should still go to exile, not graveyard");
}

/// A flashback spell that fizzles should NOT emit SpellResolved.
/// Before the fizzle fix, this would have emitted SpellResolved because
/// on_resolve was always called — it just happened to not deal damage
/// because the target was gone. The spell was treated as "resolved" when
/// it should have been "countered by game rules."
#[test]
fn flashback_spell_fizzle_no_resolved_event() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let geistflame = state.create_object(gf_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(geistflame).unwrap().name = "Geistflame".into();

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let creature = ready_creature(&mut state, P1, 2, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: geistflame, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Target dies before resolution.
    state.move_object(creature, Zone::Graveyard);
    state.events.clear();

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should NOT emit SpellResolved — the spell fizzled.
    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { object } if *object == geistflame)
    });
    assert!(!has_resolved,
        "Flashback spell that fizzles should NOT emit SpellResolved — \
         it was countered by game rules, not resolved");

    // Should still be in exile (flashback replacement applies to fizzled spells too).
    assert_eq!(state.get_object(geistflame).unwrap().zone, Zone::Exile);

    // P1 should not have taken damage.
    assert_eq!(state.get_player(P1).life, 20);
}

// ════════════════════════════════════════════════════════════════════
// Counterspell fizzle: target spell already left the stack
// ════════════════════════════════════════════════════════════════════

/// Counterspell targeting a spell that's already been removed from the stack.
/// The Counterspell should fizzle — no SpellResolved event.
#[test]
fn counterspell_fizzle_no_resolved_event() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)] },
        &reg,
    );

    state.priority_player = Some(P1);
    let counter = castable_spell(&mut state, &reg, "Counterspell", P1);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: counter, targets: vec![Target::Object(bolt)] },
        &reg,
    );

    // Bolt removed from stack before Counterspell resolves.
    state.stack.retain(|&id| id != bolt);
    state.move_object(bolt, Zone::Graveyard);
    state.events.clear();

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { object } if *object == counter)
    });
    assert!(!has_resolved,
        "Counterspell with illegal target should fizzle, not resolve");
}

// ════════════════════════════════════════════════════════════════════
// No-target spells should never fizzle
// ════════════════════════════════════════════════════════════════════

/// Spells with no targets (like Divination) can never fizzle.
#[test]
fn no_target_spell_always_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a library to draw from.
    let card_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..5 {
        let id = state.create_object(card_id, P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(id);
    }

    let div = castable_spell(&mut state, &reg, "Divination", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: div, targets: vec![] },
        &reg,
    );
    state.events.clear();

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { .. })
    });
    assert!(has_resolved, "Spell with no targets should always resolve");
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2,
        "Divination should draw 2 cards");
}

// ════════════════════════════════════════════════════════════════════
// Player targets never become illegal (players can't leave the game
// mid-spell in our 2-player implementation)
// ════════════════════════════════════════════════════════════════════

/// A spell targeting a player should always resolve (players don't become
/// illegal targets in a 2-player game).
#[test]
fn player_target_never_fizzles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)] },
        &reg,
    );
    state.events.clear();

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let has_resolved = state.events.iter().any(|e| {
        matches!(e, GameEvent::SpellResolved { .. })
    });
    assert!(has_resolved, "Spell targeting a player should resolve normally");
    assert_eq!(state.get_player(P1).life, 17, "Bolt should deal 3 damage");
}

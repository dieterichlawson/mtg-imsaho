//! Tests for Creepy Doll.
//!
//! Oracle: {5} 1/1 Artifact Creature — Construct
//! Indestructible
//! Whenever Creepy Doll deals combat damage to a creature, flip a coin.
//! If you win the flip, destroy that creature.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::{DamageTarget, GameEvent};
use mtg_engine::state::StackEntry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Creepy Doll should have DealsCombatDamageToCreature trigger, not Blocks/BecomesBlocked.
#[test]
fn has_correct_trigger_kind() {
    let reg = registry();
    let id = reg.get_id_by_name("Creepy Doll").unwrap();
    let data = reg.card_data(id).unwrap();

    let trigger_kinds: Vec<_> = data.triggered_abilities.iter()
        .map(|t| &t.kind)
        .collect();

    assert!(trigger_kinds.iter().any(|k| matches!(k, mtg_engine::cards::TriggerKind::DealsCombatDamageToCreature)),
        "Should have DealsCombatDamageToCreature trigger");
    assert!(!trigger_kinds.iter().any(|k| matches!(k, mtg_engine::cards::TriggerKind::Blocks)),
        "Should NOT have Blocks trigger");
    assert!(!trigger_kinds.iter().any(|k| matches!(k, mtg_engine::cards::TriggerKind::BecomesBlocked)),
        "Should NOT have BecomesBlocked trigger");
}

/// Creepy Doll should have indestructible.
#[test]
fn has_indestructible() {
    let reg = registry();
    let id = reg.get_id_by_name("Creepy Doll").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.keywords.contains(&Keyword::Indestructible));
}

/// The trigger should fire when CombatDamageDealt event targets a creature.
#[test]
fn trigger_fires_on_combat_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    let doll = named_creature(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Set up combat: doll attacks, target blocks.
    state.combat.as_mut().unwrap().attackers.insert(doll, P1);

    // Emit combat damage event (doll deals 1 damage to target creature).
    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });

    // Collect triggers.
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // Should have a trigger on the stack for Creepy Doll's ability.
    let has_trigger = state.stack.iter().any(|entry| matches!(entry, StackEntry::Trigger(_)));
    assert!(has_trigger,
        "Should have a trigger on the stack for Creepy Doll's combat damage to creature");
}

/// The trigger should NOT fire when CombatDamageDealt targets a player.
#[test]
fn trigger_does_not_fire_on_combat_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    let doll = named_creature(&mut state, &reg, "Creepy Doll", P0);
    state.combat.as_mut().unwrap().attackers.insert(doll, P1);

    // Emit combat damage event (doll deals 1 damage to player).
    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Player(P1),
        amount: 1,
    });

    // Collect triggers.
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // Should NOT have a trigger on the stack (Creepy Doll doesn't have CombatDamageToPlayer).
    let has_trigger = state.stack.iter().any(|entry| matches!(entry, StackEntry::Trigger(_)));
    assert!(!has_trigger,
        "Should NOT trigger on combat damage to player");
}

/// The on_deals_combat_damage_to_creature hook calls try_destroy on win.
#[test]
fn on_deals_combat_damage_to_creature_calls_destroy() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_creature(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Call the hook directly many times to verify it can destroy.
    // (Due to randomness, we call it many times and check that at least one destroys.)
    let card_id = state.get_object(doll).unwrap().card_id;
    let behavior = reg.get(card_id).unwrap();

    let mut any_destroyed = false;
    for _ in 0..50 {
        let mut test_state = state.clone();
        behavior.on_deals_combat_damage_to_creature(&mut test_state, doll, target, 1, &reg);
        if test_state.get_object(target).map(|o| o.zone != Zone::Battlefield).unwrap_or(false) {
            any_destroyed = true;
            break;
        }
    }
    assert!(any_destroyed, "Creepy Doll should eventually destroy the target creature");
}

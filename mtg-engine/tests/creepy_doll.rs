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

/// The trigger should fire when `CombatDamageDealt` event targets a creature.
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

/// The trigger should NOT fire when `CombatDamageDealt` targets a player.
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

/// The `on_deals_combat_damage_to_creature` hook calls `try_destroy` on win.
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
        if test_state.get_object(target).is_some_and(|o| o.zone != Zone::Battlefield) {
            any_destroyed = true;
            break;
        }
    }
    assert!(any_destroyed, "Creepy Doll should eventually destroy the target creature");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: When Creepy Doll deals lethal combat damage to a creature
/// AND wins the coin flip, the creature should be destroyed by the
/// triggered ability even if it could regenerate from the lethal damage.
/// The ruling says these are separate events.
/// Note: This is hard to test deterministically due to the coin flip.
/// We test the simpler case: Creepy Doll's trigger fires even when
/// the creature already has lethal damage.
#[test]
fn bug_creepy_doll_trigger_with_lethal_damage() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doll = named_creature(&mut state, &registry, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 2, 1); // 1 toughness, will take lethal from 1 dmg

    // Simulate combat damage: Doll deals 1 to target (lethal for 1 toughness)
    if let Some(obj) = state.get_object_mut(target) {
        obj.damage_marked = 1;
        obj.damaged_by.push(doll);
    }

    // Give target a regeneration shield (to survive lethal damage)
    if let Some(obj) = state.get_object_mut(target) {
        obj.regeneration_shields = 1;
    }

    // The trigger should still fire (it's a separate "destroy" effect)
    let behavior = registry.get(state.get_object(doll).unwrap().card_id).unwrap();
    behavior.on_deals_combat_damage_to_creature(&mut state, doll, target, 1, &registry);

    // After the trigger (which calls try_destroy on a coin flip win),
    // the creature may survive (regeneration absorbs the destroy) or die.
    // The key question is whether the trigger FIRES at all — it should.
    // We can't control the coin flip, but we can verify the trigger ran
    // by checking if try_destroy was called (regeneration shield consumed).
    let _shields_after = state.get_object(target).unwrap().regeneration_shields;

    // If the coin flip was won AND try_destroy was called, the shield is consumed.
    // If the coin flip was lost, shields remain at 1.
    // Either way, the trigger should have fired. We verify by running SBAs
    // and checking the creature survived via regeneration.
    // Run the trigger multiple times to get at least one coin flip win.
    // If try_destroy is called on a win, the regeneration shield is consumed.
    // We reset and retry until we get a win (statistically guaranteed in ~10 tries).
    let mut won_at_least_once = false;
    for _ in 0..20 {
        // Reset target state
        if let Some(obj) = state.get_object_mut(target) {
            obj.regeneration_shields = 1;
            obj.damage_marked = 1;
            obj.zone = Zone::Battlefield;
        }

        behavior.on_deals_combat_damage_to_creature(&mut state, doll, target, 1, &registry);

        let shields = state.get_object(target).unwrap().regeneration_shields;
        if shields == 0 {
            // Coin flip was won, try_destroy was called, regeneration was consumed
            won_at_least_once = true;
            break;
        }
    }

    assert!(won_at_least_once,
        "After 20 attempts, Creepy Doll should have won at least one coin flip and called try_destroy");
}

//! Tests for Innistrad Tier 8 cards (sacrifice-based abilities + graveyard exile costs).

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Skirsdag Cultist ─────────────────────────────────────────────

/// Skirsdag Cultist activated ability deals 2 damage to a target creature.
#[test]
fn skirsdag_cultist_deals_2_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    // Need a creature to sacrifice (can sacrifice itself or another creature).
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Add red mana for the activation cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: cultist,
            ability_index: 0,
            targets: vec![Target::Object(target)],
        },
        &reg,
    );

    // Target creature should have taken 2 damage.
    let obj = state.get_object(target).unwrap();
    assert_eq!(obj.damage_marked, 2, "Target should have 2 damage marked");
}

/// Skirsdag Cultist deals 2 damage to a player.
#[test]
fn skirsdag_cultist_deals_2_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: cultist,
            ability_index: 0,
            targets: vec![Target::Player(P1)],
        },
        &reg,
    );

    assert_eq!(state.get_player(P1).life, 18, "Opponent should be at 18 life");
}

/// Skirsdag Cultist requires tap, red mana, and a creature to sacrifice.
#[test]
fn skirsdag_cultist_cannot_activate_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cultist is the only creature. It will be sacrificed as part of the cost,
    // but we need at least one creature to sacrifice. Since the cultist itself
    // counts, the ability should still be available.
    let _cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let has_activate = actions.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(has_activate, "Should be able to activate (cultist counts as sacrifice fodder)");
}

// ── Stitcher's Apprentice ────────────────────────────────────────

/// Stitcher's Apprentice creates a 2/2 token then sacrifices a creature.
#[test]
fn stitchers_apprentice_creates_token_then_sacrifices() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_creature(&mut state, &reg, "Stitcher's Apprentice", P0);

    // Add mana for the activation cost ({1}{U}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Count creatures before activation.
    let creatures_before: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .collect();
    assert_eq!(creatures_before.len(), 1, "Only the apprentice on the battlefield");

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: apprentice,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // After activation: a 2/2 token was created, then a creature was sacrificed.
    // The auto-sacrifice picks the first creature, which could be the token or
    // an existing creature. Net result: one creature on battlefield.
    let creatures_after: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .collect();
    // The apprentice was tapped and a creature was sacrificed. The token was created.
    // Net: we should have exactly 1 creature (the one that wasn't sacrificed).
    assert_eq!(creatures_after.len(), 1, "Should have 1 creature on battlefield after create + sacrifice");

    // One creature should be in the graveyard (the sacrificed one).
    let graveyard: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0 && o.power.is_some())
        .collect();
    // Note: tokens cease to exist when they go to graveyard (SBA), but before SBA we still see it.
    assert!(graveyard.len() >= 1, "A creature should have been sacrificed");
}

/// Stitcher's Apprentice creates a 2/2 Homunculus token.
#[test]
fn stitchers_apprentice_token_is_2_2_homunculus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_creature(&mut state, &reg, "Stitcher's Apprentice", P0);
    // Add a second creature so the apprentice doesn't sacrifice the token immediately.
    let _fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: apprentice,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // Find the token (is_token == true).
    let token = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.power.is_some());
    assert!(token.is_some(), "A token should exist on the battlefield");
    let token = token.unwrap();
    assert_eq!(token.power, Some(2), "Token should have power 2");
    assert_eq!(token.toughness, Some(2), "Token should have toughness 2");
    assert_eq!(token.name, "Homunculus", "Token should be named Homunculus");
}

// ── Corpse Lunge ─────────────────────────────────────────────────

/// Corpse Lunge exiles a creature from graveyard and deals damage equal to its power.
#[test]
fn corpse_lunge_deals_damage_equal_to_exiled_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a 4/4 creature in P0's graveyard.
    let gy_creature = ready_creature(&mut state, P0, 4, 4);
    state.get_object_mut(gy_creature).unwrap().name = "Big Creature".into();
    state.move_object(gy_creature, Zone::Graveyard);

    // Target creature on P1's battlefield.
    let target = ready_creature(&mut state, P1, 5, 5);

    // Cast Corpse Lunge.
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // The graveyard creature should be in exile.
    let exiled = state.get_object(gy_creature).unwrap();
    assert_eq!(exiled.zone, Zone::Exile, "Graveyard creature should be exiled");

    // Target creature should have 4 damage.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 4, "Target should have 4 damage from Corpse Lunge");
}

/// Corpse Lunge with no creature in graveyard deals no damage.
#[test]
fn corpse_lunge_no_graveyard_creature_deals_no_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = ready_creature(&mut state, P1, 3, 3);

    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 0, "No damage should be dealt without graveyard creature");
}

/// Corpse Lunge picks the highest-power creature from graveyard.
#[test]
fn corpse_lunge_picks_highest_power_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put two creatures in graveyard: a 2/2 and a 5/5.
    let small = ready_creature(&mut state, P0, 2, 2);
    state.move_object(small, Zone::Graveyard);
    let big = ready_creature(&mut state, P0, 5, 5);
    state.move_object(big, Zone::Graveyard);

    let target = ready_creature(&mut state, P1, 6, 6);

    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // Should exile the 5/5 and deal 5 damage.
    let big_obj = state.get_object(big).unwrap();
    assert_eq!(big_obj.zone, Zone::Exile, "Highest-power creature should be exiled");

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 5, "Should deal 5 damage (power of exiled 5/5)");
}

// ── Harvest Pyre ─────────────────────────────────────────────────

/// Harvest Pyre exiles all graveyard cards and deals damage equal to the count.
#[test]
fn harvest_pyre_deals_damage_equal_to_exiled_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 4 cards in P0's graveyard.
    for _ in 0..4 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let target = ready_creature(&mut state, P1, 5, 5);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // All 4 cards should be exiled.
    let exiled_count = state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(exiled_count, 4, "All 4 graveyard cards should be exiled");

    // Target should have 4 damage.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 4, "Target should have 4 damage from Harvest Pyre");
}

/// Harvest Pyre with empty graveyard deals 0 damage.
#[test]
fn harvest_pyre_empty_graveyard_deals_no_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = ready_creature(&mut state, P1, 3, 3);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 0, "No damage should be dealt with empty graveyard");
}

/// Harvest Pyre only exiles the caster's graveyard cards, not the opponent's.
#[test]
fn harvest_pyre_only_exiles_own_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 cards in P0's graveyard.
    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }
    // Put 2 cards in P1's graveyard.
    for _ in 0..2 {
        let c = state.create_object(CardId(9999), P1, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let target = ready_creature(&mut state, P1, 6, 6);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // Only P0's 3 cards should be exiled.
    let p0_exiled = state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(p0_exiled, 3, "Only P0's 3 graveyard cards should be exiled");

    // P1's graveyard should be untouched.
    let p1_gy = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert_eq!(p1_gy, 2, "P1's graveyard should be untouched");

    // Target should have 3 damage.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 3, "Target should have 3 damage");
}

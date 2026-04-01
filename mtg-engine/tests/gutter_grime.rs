//! Tests for Gutter Grime — dynamic Ooze token P/T.
//!
//! Oracle: "Whenever a nontoken creature you control dies, put a slime counter on
//! Gutter Grime, then create a green Ooze creature token with 'This creature's
//! power and toughness are each equal to the number of slime counters on Gutter Grime.'"
//!
//! Key behaviors:
//! - Ooze token P/T dynamically tracks slime counter count on source Gutter Grime
//! - Adding more slime counters makes ALL existing Ooze tokens bigger
//! - Only nontoken creatures trigger it
//! - Only creatures you control trigger it

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::CardId;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// When a nontoken creature dies, Gutter Grime should create an Ooze token
/// whose P/T dynamically equals the slime counter count.
#[test]
fn gutter_grime_creates_dynamic_pt_ooze() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Gutter Grime on the battlefield.
    let gutter_grime = named_creature(&mut state, &reg, "Gutter Grime", P0);

    // Put a nontoken creature on the battlefield for P0.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().card_types = vec![CardType::Creature];

    // Simulate the creature dying.
    let card_id = state.get_object(creature).unwrap().card_id;
    state.move_object(creature, Zone::Graveyard);
    state.events.push(GameEvent::CreatureDied {
        object: creature,
        card_id,
        controller: P0,
        damaged_by: vec![],
        last_known_toughness: 2,
    });

    // Process triggers.
    triggers::process_triggers(&mut state, &reg);

    // Gutter Grime should have 1 slime counter.
    let slime_count = *state.get_object(gutter_grime).unwrap()
        .counters.get(&CounterType::Slime).unwrap_or(&0);
    assert_eq!(slime_count, 1, "Gutter Grime should have 1 slime counter");

    // Find the Ooze token.
    let ooze_tokens: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Ooze")
        .collect();
    assert_eq!(ooze_tokens.len(), 1, "Should have created 1 Ooze token");

    // The Ooze token should have dynamic P/T equal to slime counter count (1).
    let ooze_id = ooze_tokens[0].id;
    let eff_power = state.effective_power(ooze_id, &reg).unwrap();
    let eff_toughness = state.effective_toughness(ooze_id, &reg).unwrap();
    assert_eq!(eff_power, 1, "Ooze effective power should be 1 (1 slime counter)");
    assert_eq!(eff_toughness, 1, "Ooze effective toughness should be 1 (1 slime counter)");
}

/// When more creatures die, all Ooze tokens should grow as slime counters increase.
#[test]
fn gutter_grime_ooze_tokens_grow_with_more_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_creature(&mut state, &reg, "Gutter Grime", P0);

    // Kill first creature.
    let creature1 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature1).unwrap().card_types = vec![CardType::Creature];
    let card_id1 = state.get_object(creature1).unwrap().card_id;
    state.move_object(creature1, Zone::Graveyard);
    state.events.push(GameEvent::CreatureDied {
        object: creature1, card_id: card_id1, controller: P0,
        damaged_by: vec![], last_known_toughness: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    // Clear events so the second death is processed cleanly.
    state.events.clear();
    state.trigger_event_index = 0;

    // Kill second creature.
    let creature2 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature2).unwrap().card_types = vec![CardType::Creature];
    let card_id2 = state.get_object(creature2).unwrap().card_id;
    state.move_object(creature2, Zone::Graveyard);
    state.events.push(GameEvent::CreatureDied {
        object: creature2, card_id: card_id2, controller: P0,
        damaged_by: vec![], last_known_toughness: 3,
    });
    triggers::process_triggers(&mut state, &reg);

    // Gutter Grime should have 2 slime counters now.
    let slime_count = *state.get_object(gutter_grime).unwrap()
        .counters.get(&CounterType::Slime).unwrap_or(&0);
    assert_eq!(slime_count, 2, "Gutter Grime should have 2 slime counters");

    // Find all Ooze tokens.
    let ooze_tokens: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Ooze")
        .collect();
    assert_eq!(ooze_tokens.len(), 2, "Should have 2 Ooze tokens");

    // ALL Ooze tokens should have effective P/T = 2 (both track current slime count).
    for ooze in &ooze_tokens {
        let eff_power = state.effective_power(ooze.id, &reg).unwrap();
        let eff_toughness = state.effective_toughness(ooze.id, &reg).unwrap();
        assert_eq!(eff_power, 2,
            "All Ooze tokens should have effective power 2 (2 slime counters)");
        assert_eq!(eff_toughness, 2,
            "All Ooze tokens should have effective toughness 2 (2 slime counters)");
    }
}

/// Token creatures dying should NOT trigger Gutter Grime.
#[test]
fn gutter_grime_ignores_token_deaths() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_creature(&mut state, &reg, "Gutter Grime", P0);

    // Create a token creature.
    let token_id = state.create_token("Spirit", P0, 1, 1,
        vec![Color::White], vec![CardType::Creature], vec![]);

    // Kill the token.
    let card_id = state.get_object(token_id).unwrap().card_id;
    state.move_object(token_id, Zone::Graveyard);
    state.events.push(GameEvent::CreatureDied {
        object: token_id, card_id, controller: P0,
        damaged_by: vec![], last_known_toughness: 1,
    });
    triggers::process_triggers(&mut state, &reg);

    // Gutter Grime should have 0 slime counters.
    let slime_count = *state.get_object(gutter_grime).unwrap()
        .counters.get(&CounterType::Slime).unwrap_or(&0);
    assert_eq!(slime_count, 0, "Gutter Grime should not trigger on token deaths");
}

/// Opponent's creatures dying should NOT trigger Gutter Grime.
#[test]
fn gutter_grime_ignores_opponent_deaths() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_creature(&mut state, &reg, "Gutter Grime", P0);

    // Create opponent's creature.
    let opp_creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(opp_creature).unwrap().card_types = vec![CardType::Creature];
    let card_id = state.get_object(opp_creature).unwrap().card_id;
    state.move_object(opp_creature, Zone::Graveyard);
    state.events.push(GameEvent::CreatureDied {
        object: opp_creature, card_id, controller: P1,
        damaged_by: vec![], last_known_toughness: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    let slime_count = *state.get_object(gutter_grime).unwrap()
        .counters.get(&CounterType::Slime).unwrap_or(&0);
    assert_eq!(slime_count, 0, "Gutter Grime should not trigger on opponent's creature deaths");
}

/// If Gutter Grime leaves the battlefield, existing Ooze tokens should become 0/0.
#[test]
fn gutter_grime_ooze_tokens_become_zero_without_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_creature(&mut state, &reg, "Gutter Grime", P0);

    // Kill a creature to create an Ooze.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().card_types = vec![CardType::Creature];
    let card_id = state.get_object(creature).unwrap().card_id;
    state.move_object(creature, Zone::Graveyard);
    state.events.push(GameEvent::CreatureDied {
        object: creature, card_id, controller: P0,
        damaged_by: vec![], last_known_toughness: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    let ooze_id = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Ooze")
        .unwrap().id;

    // Remove Gutter Grime from battlefield.
    state.move_object(gutter_grime, Zone::Graveyard);

    // The Ooze token should now have effective P/T = 0 (source has no slime counters visible).
    let eff_power = state.effective_power(ooze_id, &reg).unwrap();
    let eff_toughness = state.effective_toughness(ooze_id, &reg).unwrap();
    assert_eq!(eff_power, 0, "Ooze should be 0/0 when Gutter Grime leaves battlefield");
    assert_eq!(eff_toughness, 0, "Ooze should be 0/0 when Gutter Grime leaves battlefield");
}

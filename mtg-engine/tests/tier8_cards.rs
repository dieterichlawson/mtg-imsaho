//! Tests for Innistrad Tier 8 cards (sacrifice-as-cost and additional casting costs).

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Altar's Reap ──────────────────────────────────────────────────

/// Altar's Reap sacrifices a creature and draws two cards.
#[test]
fn altars_reap_sacrifices_and_draws_two() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a creature to sacrifice and cards to draw.
    let creature = ready_creature(&mut state, P0, 2, 2);
    for _ in 0..3 {
        let c = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(c);
    }

    let spell = castable_spell(&mut state, &reg, "Altar's Reap", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Creature should be in graveyard (sacrificed).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Should have drawn 2 cards (hand was empty before spell was added).
    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, 2,
        "Should have drawn 2 cards");
}

// ── Infernal Plunge ───────────────────────────────────────────────

/// Infernal Plunge sacrifices a creature and adds {R}{R}{R}.
#[test]
fn infernal_plunge_sacrifices_and_adds_rrr() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a creature to sacrifice.
    let creature = ready_creature(&mut state, P0, 1, 1);

    let spell = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Creature should be sacrificed.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Should have 3 red mana in pool (the {R} used to cast was consumed, then {R}{R}{R} added).
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Red), 3,
        "Should have 3 red mana in pool after Infernal Plunge");
}

// ── Tribute to Hunger ─────────────────────────────────────────────

/// Tribute to Hunger: opponent sacrifices a creature, caster gains life.
/// With exactly one opponent creature, it auto-sacrifices.
#[test]
fn tribute_to_hunger_opponent_sacs_and_gain_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give the opponent a 3/4 creature.
    let opp_creature = ready_creature(&mut state, P1, 3, 4);
    // Set the creature's name for logging.
    state.get_object_mut(opp_creature).unwrap().name = "Big Beast".into();

    let initial_life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Player(P1)]);

    // Opponent's creature should be sacrificed.
    assert_eq!(state.get_object(opp_creature).unwrap().zone, Zone::Graveyard,
        "Opponent's creature should be sacrificed");

    // Caster should have gained life equal to toughness (4).
    assert_eq!(state.get_player(P0).life, initial_life + 4,
        "Should have gained life equal to sacrificed creature's toughness");
}

/// Tribute to Hunger with no opponent creatures does nothing extra.
#[test]
fn tribute_to_hunger_no_creatures_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let initial_life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Player(P1)]);

    // No life change since no creature was sacrificed.
    assert_eq!(state.get_player(P0).life, initial_life,
        "No life gain when opponent has no creatures");
}

// ── Divine Reckoning ──────────────────────────────────────────────

/// Divine Reckoning: each player keeps one creature, the rest are sacrificed.
#[test]
fn divine_reckoning_keeps_one_per_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has 3 creatures: 2/1, 2/3, 1/5.
    let _c1 = ready_creature(&mut state, P0, 2, 1);
    let _c2 = ready_creature(&mut state, P0, 2, 3);
    let c3 = ready_creature(&mut state, P0, 1, 5); // highest toughness — should be kept

    // P1 has 2 creatures: 4/2, 3/4.
    let _c4 = ready_creature(&mut state, P1, 4, 2);
    let c5 = ready_creature(&mut state, P1, 3, 4); // highest toughness — should be kept

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // P0 should have exactly 1 creature left on battlefield.
    let p0_creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.power.is_some())
        .collect();
    assert_eq!(p0_creatures.len(), 1, "P0 should have exactly 1 creature");
    assert_eq!(p0_creatures[0].id, c3, "P0 should keep the highest-toughness creature");

    // P1 should have exactly 1 creature left on battlefield.
    let p1_creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.power.is_some())
        .collect();
    assert_eq!(p1_creatures.len(), 1, "P1 should have exactly 1 creature");
    assert_eq!(p1_creatures[0].id, c5, "P1 should keep the highest-toughness creature");
}

/// Divine Reckoning with 0 or 1 creature per player does nothing.
#[test]
fn divine_reckoning_with_one_creature_keeps_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has 1 creature.
    let c1 = ready_creature(&mut state, P0, 3, 3);
    // P1 has 0 creatures.

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // P0's single creature should still be on the battlefield.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield,
        "Single creature should survive Divine Reckoning");

    // P1 should have no creatures (had none to begin with).
    let p1_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.power.is_some())
        .count();
    assert_eq!(p1_creatures, 0, "P1 should have no creatures");
}

/// Divine Reckoning has flashback {5}{W}{W}.
#[test]
fn divine_reckoning_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Divine Reckoning").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some(), "Divine Reckoning should have flashback");
    assert_eq!(data.flashback_cost.unwrap().mana_value(), 7, "Flashback should cost 5WW = 7 MV");
}

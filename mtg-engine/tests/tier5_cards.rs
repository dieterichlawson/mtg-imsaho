//! Tests for Innistrad Tier 5 cards.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Spirit lords ───────────────────────────────────────────────────

/// Battleground Geist gives other Spirits +1/+0 but not itself.
#[test]
fn battleground_geist_buffs_other_spirits() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let geist = named_creature(&mut state, &reg, "Battleground Geist", P0);
    let spirit = named_creature(&mut state, &reg, "Chapel Geist", P0);
    let non_spirit = ready_creature(&mut state, P0, 2, 2);

    // Battleground Geist: 3/3 base. Should NOT buff itself ("other").
    assert_eq!(state.effective_power(geist, &reg), Some(3));
    // Chapel Geist: 2/3 base + 1/0 from lord = 3/3.
    assert_eq!(state.effective_power(spirit, &reg), Some(3));
    assert_eq!(state.effective_toughness(spirit, &reg), Some(3));
    // Non-spirit: no buff.
    assert_eq!(state.effective_power(non_spirit, &reg), Some(2));
}

/// Gallows Warden gives other Spirits +0/+1 but not itself.
#[test]
fn gallows_warden_buffs_other_spirits() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let warden = named_creature(&mut state, &reg, "Gallows Warden", P0);
    let spirit = named_creature(&mut state, &reg, "Chapel Geist", P0);

    // Warden: 3/3 base, should NOT buff itself.
    assert_eq!(state.effective_toughness(warden, &reg), Some(3));
    // Chapel Geist: 2/3 base + 0/1 from warden = 2/4.
    assert_eq!(state.effective_toughness(spirit, &reg), Some(4));
    assert_eq!(state.effective_power(spirit, &reg), Some(2));
}

/// Spirit lords don't buff opponent's spirits.
#[test]
fn spirit_lord_doesnt_buff_opponent() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _geist = named_creature(&mut state, &reg, "Battleground Geist", P0);
    let enemy_spirit = named_creature(&mut state, &reg, "Chapel Geist", P1);

    // Opponent's spirit should NOT get the buff.
    assert_eq!(state.effective_power(enemy_spirit, &reg), Some(2));
}

// ── Dynamic P/T ────────────────────────────────────────────────────

/// Geist-Honored Monk P/T equals creatures you control, ETB creates 2 tokens.
#[test]
fn geist_honored_monk_dynamic_pt_and_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monk = castable_spell(&mut state, &reg, "Geist-Honored Monk", P0);

    // Give P0 a library so draw doesn't fail.
    let land_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..5 {
        let id = state.create_object(land_id, P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(id);
    }

    state = cast_and_resolve(&state, &reg, monk, vec![]);
    triggers::process_triggers(&mut state, &reg);

    // Monk + 2 Spirit tokens = 3 creatures.
    let creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.power.is_some())
        .collect();
    assert_eq!(creatures.len(), 3, "Monk + 2 Spirit tokens");

    // Monk's P/T should be 3/3 (3 creatures you control).
    assert_eq!(state.effective_power(monk, &reg), Some(3));
    assert_eq!(state.effective_toughness(monk, &reg), Some(3));
}

/// Wreath of Geists gives +X/+X based on creature cards in graveyard.
#[test]
fn wreath_of_geists_dynamic_buff() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    // Put 3 creature cards in P0's graveyard.
    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1));
        state.get_object_mut(c).unwrap().name = "Dead Creature".into();
    }

    let wreath = castable_spell(&mut state, &reg, "Wreath of Geists", P0);
    state = cast_and_resolve(&state, &reg, wreath, vec![Target::Object(creature)]);

    // 2/2 base + 3/3 from Wreath (3 creatures in graveyard) = 5/5.
    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(5));
}

/// Wreath of Geists updates dynamically as graveyard changes.
#[test]
fn wreath_of_geists_updates_dynamically() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    let wreath = castable_spell(&mut state, &reg, "Wreath of Geists", P0);
    state = cast_and_resolve(&state, &reg, wreath, vec![Target::Object(creature)]);

    // No creatures in graveyard yet: 2/2 + 0/0 = 2/2.
    assert_eq!(state.effective_power(creature, &reg), Some(2));

    // Add a creature to graveyard: 2/2 + 1/1 = 3/3.
    let dead = state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1));
    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));
}

// ── Block restriction ──────────────────────────────────────────────

/// Orchard Spirit can't be blocked by ground creatures.
#[test]
fn orchard_spirit_not_blocked_by_ground() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let spirit = named_creature(&mut state, &reg, "Orchard Spirit", P0);
    let ground = ready_creature(&mut state, P1, 3, 3);

    assert!(!combat::can_block_attacker(&state, ground, spirit, &reg),
        "Ground creature should not be able to block Orchard Spirit");
}

/// Orchard Spirit CAN be blocked by a creature with flying.
#[test]
fn orchard_spirit_blocked_by_flyer() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let spirit = named_creature(&mut state, &reg, "Orchard Spirit", P0);
    let flyer = named_creature(&mut state, &reg, "Chapel Geist", P1);

    assert!(combat::can_block_attacker(&state, flyer, spirit, &reg),
        "Flyer should be able to block Orchard Spirit");
}

/// Orchard Spirit CAN be blocked by a creature with reach.
#[test]
fn orchard_spirit_blocked_by_reach() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let spirit = named_creature(&mut state, &reg, "Orchard Spirit", P0);
    let spider = named_creature(&mut state, &reg, "Somberwald Spider", P1);

    assert!(combat::can_block_attacker(&state, spider, spirit, &reg),
        "Reach creature should be able to block Orchard Spirit");
}

// ── Token creation from graveyard ──────────────────────────────────

/// Spider Spawning creates tokens equal to creatures in graveyard.
#[test]
fn spider_spawning_creates_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 4 creature cards in graveyard.
    for i in 0..4 {
        let c = state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1));
        state.get_object_mut(c).unwrap().name = format!("Dead {}", i);
    }

    let ss = castable_spell(&mut state, &reg, "Spider Spawning", P0);
    state = cast_and_resolve(&state, &reg, ss, vec![]);

    // Should have 4 Spider tokens on the battlefield.
    let spiders: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Spider" && o.is_token)
        .collect();
    assert_eq!(spiders.len(), 4, "Should create 4 Spider tokens");

    // Each should be 1/2 with reach.
    for spider in &spiders {
        assert_eq!(spider.power, Some(1));
        assert_eq!(spider.toughness, Some(2));
    }
}

// ── Morbid ETB ─────────────────────────────────────────────────────

/// Festerhide Boar gets +1/+1 counters with morbid.
#[test]
fn festerhide_boar_morbid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let boar = castable_spell(&mut state, &reg, "Festerhide Boar", P0);
    state = cast_and_resolve(&state, &reg, boar, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let counters = state.get_counter_count(boar, CounterType::PlusOnePlusOne);
    assert_eq!(counters, 2, "Morbid Festerhide Boar should have 2 +1/+1 counters");
    // 3/3 base + 2/2 from counters = 5/5.
    assert_eq!(state.effective_power(boar, &reg), Some(5));
}

/// Festerhide Boar without morbid — no counters.
#[test]
fn festerhide_boar_no_morbid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let boar = castable_spell(&mut state, &reg, "Festerhide Boar", P0);
    state = cast_and_resolve(&state, &reg, boar, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let counters = state.get_counter_count(boar, CounterType::PlusOnePlusOne);
    assert_eq!(counters, 0, "No morbid = no counters");
    assert_eq!(state.effective_power(boar, &reg), Some(3));
}

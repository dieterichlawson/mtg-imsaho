//! Tests for Innistrad Tier 7 cards (upkeep/end-step triggers + curses).

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Boneyard Wurm ─────────────────────────────────────────────────

/// Boneyard Wurm P/T = creature cards in graveyard.
#[test]
fn boneyard_wurm_pt_equals_creatures_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wurm = named_creature(&mut state, &reg, "Boneyard Wurm", P0);

    // No creatures in graveyard yet.
    assert_eq!(state.effective_power(wurm, &reg).unwrap(), 0);
    assert_eq!(state.effective_toughness(wurm, &reg).unwrap(), 0);

    // Put 3 creatures in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    assert_eq!(state.effective_power(wurm, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(wurm, &reg).unwrap(), 3);
}

// ── Splinterfright ────────────────────────────────────────────────

/// Splinterfright mills 2 on upkeep.
#[test]
fn splinterfright_mills_on_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _splinter = named_creature(&mut state, &reg, "Splinterfright", P0);

    // Put cards in library.
    for _ in 0..4 {
        let c = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(c);
    }

    // Fire upkeep trigger.
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Should have milled 2.
    let gy_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0)
        .count();
    assert_eq!(gy_count, 2, "Splinterfright should mill 2 cards on upkeep");
}

// ── Bloodgift Demon ───────────────────────────────────────────────

/// Bloodgift Demon draws a card and loses 1 life on upkeep.
#[test]
fn bloodgift_demon_draws_and_loses_life() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _demon = named_creature(&mut state, &reg, "Bloodgift Demon", P0);

    // Library card to draw.
    let lib = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Bloodgift Demon now presents a player choice. Choose P0 (self).
    assert!(state.awaiting_action.is_some(), "Should be awaiting player choice");
    let state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(mtg_engine::actions::Target::Player(P0))),
        },
        &reg,
    );

    let hand = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand, 1, "Should have drawn 1 card");
    assert_eq!(state.get_player(P0).life, 19, "Should have lost 1 life");
}

// ── Endless Ranks of the Dead ─────────────────────────────────────

/// Creates Zombie tokens equal to half your Zombies.
#[test]
fn endless_ranks_creates_zombie_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _enchantment = named_creature(&mut state, &reg, "Endless Ranks of the Dead", P0);

    // Create 5 Zombies on the battlefield.
    for _ in 0..5 {
        let z = state.create_token_with_subtypes(
            "Zombie", P0, 2, 2, vec![Color::Black],
            vec![CardType::Creature], vec![], vec!["Zombie".into()],
            &reg,
        )[0];
        state.get_object_mut(z).unwrap().summoning_sick = false;
    }

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // 5 / 2 = 2 (rounded down). So 5 original + 2 new = 7 Zombies.
    let zombie_count = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.subtypes.iter().any(|s| s == "Zombie"))
        .count();
    assert_eq!(zombie_count, 7, "Should create 2 Zombie tokens (5/2 = 2)");
}

// ── Reaper from the Abyss ─────────────────────────────────────────

/// Morbid end step: destroys a non-Demon creature if a creature died this turn.
#[test]
fn reaper_destroys_non_demon_on_morbid_end_step() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let _reaper = named_creature(&mut state, &reg, "Reaper from the Abyss", P0);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Set morbid.
    state.creature_died_this_turn = true;

    fire_step_trigger(&mut state, Step::EndStep, &reg);

    // SBAs to move destroyed creature to graveyard.
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Graveyard,
        "Reaper should destroy a non-Demon creature");
}

/// Reaper does NOT trigger without morbid.
#[test]
fn reaper_no_trigger_without_morbid() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let _reaper = named_creature(&mut state, &reg, "Reaper from the Abyss", P0);
    let target = ready_creature(&mut state, P1, 3, 3);

    // No morbid.
    state.creature_died_this_turn = false;

    fire_step_trigger(&mut state, Step::EndStep, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Battlefield,
        "Reaper should NOT trigger without morbid");
}

// ── Curse of the Pierced Heart ────────────────────────────────────

/// Curse deals 1 damage to enchanted player on their upkeep.
#[test]
fn curse_of_pierced_heart_deals_damage_on_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1); // P1's upkeep

    // P0 controls the curse attached to P1.
    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_player(P1).life, 19, "Curse should deal 1 damage to P1");
    assert_eq!(state.get_player(P0).life, 20, "P0 should be unaffected");
}

// ── Curse of Death's Hold ─────────────────────────────────────────

/// Curse gives opponent's creatures -1/-1.
#[test]
fn curse_of_deaths_hold_debuffs_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 controls curse targeting P1.
    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Death's Hold", P0, P1);

    // P1's creature.
    let opp_creature = ready_creature(&mut state, P1, 3, 3);
    // P0's creature (should NOT be affected).
    let own_creature = ready_creature(&mut state, P0, 3, 3);

    let opp_power = state.effective_power(opp_creature, &reg).unwrap();
    let opp_toughness = state.effective_toughness(opp_creature, &reg).unwrap();
    let own_power = state.effective_power(own_creature, &reg).unwrap();

    assert_eq!(opp_power, 2, "Opponent's creature should have -1 power");
    assert_eq!(opp_toughness, 2, "Opponent's creature should have -1 toughness");
    assert_eq!(own_power, 3, "Own creature should be unaffected");
}

// ── Angel of Flight Alabaster ─────────────────────────────────────

/// Angel returns a Spirit from graveyard on upkeep (single target auto-applies).
#[test]
fn angel_of_flight_alabaster_returns_spirit() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);

    // Put a Spirit in graveyard.
    let spirit = named_creature(&mut state, &reg, "Chapel Geist", P0);
    state.move_object(spirit, Zone::Graveyard, &reg);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Single Spirit → auto-applied (mandatory with 1 target).
    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "Spirit should be returned to hand");
}

// ── Charmbreaker Devils ───────────────────────────────────────────

/// Charmbreaker Devils gets +4/+0 when you cast an instant or sorcery.
#[test]
fn charmbreaker_devils_plus4_on_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);

    // Put an instant spell on the stack and fire SpellCast event.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let spell = state.create_object(bolt_id, P0, Zone::Stack, None, None);
    state.get_object_mut(spell).unwrap().name = "Lightning Bolt".into();
    state.events.push(mtg_engine::events::GameEvent::SpellCast {
        player: P0,
        object: spell,
    });
    triggers::process_triggers(&mut state, &reg);

    let power = state.effective_power(devils, &reg).unwrap();
    assert_eq!(power, 8, "Charmbreaker Devils should be 4+4=8 power after spell cast");
}

// ── Curse of the Bloody Tome ──────────────────────────────────────

/// Curse mills 2 from enchanted player on their upkeep.
#[test]
fn curse_of_bloody_tome_mills_on_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Bloody Tome", P0, P1);

    // Put cards in P1's library.
    for _ in 0..4 {
        let c = state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order.push(c);
    }

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let gy = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert_eq!(gy, 2, "Should mill 2 cards from P1's library");
}

// ── Curse of Oblivion ─────────────────────────────────────────────

/// Curse exiles 2 cards from enchanted player's graveyard (auto when ≤2).
#[test]
fn curse_of_oblivion_exiles_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Oblivion", P0, P1);

    // Put 2 cards in P1's graveyard (auto-exiles when ≤2).
    let g1 = state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Graveyard, None, None);
    let g2 = state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Graveyard, None, None);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(g1).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(g2).unwrap().zone, Zone::Exile);
}

// ── Curse of the Nightly Hunt ─────────────────────────────────────

/// Curse forces enchanted player's creatures to attack.
#[test]
fn curse_of_nightly_hunt_forces_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P1); // P1's turn

    // P0 controls curse attached to P1.
    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Nightly Hunt", P0, P1);

    // P1's creature should be forced to attack.
    let creature = ready_creature(&mut state, P1, 2, 2);

    let has_force = state.has_continuous_effect(creature, &|e| {
        match e {
            ContinuousEffect::ForceAttack { scope } => Some(scope),
            _ => None,
        }
    }, &reg);
    assert!(has_force, "P1's creature should be forced to attack by curse");

    // P0's creature should NOT be forced.
    let own_creature = ready_creature(&mut state, P0, 2, 2);
    let own_forced = state.has_continuous_effect(own_creature, &|e| {
        match e {
            ContinuousEffect::ForceAttack { scope } => Some(scope),
            _ => None,
        }
    }, &reg);
    assert!(!own_forced, "P0's creature should NOT be forced to attack");
}

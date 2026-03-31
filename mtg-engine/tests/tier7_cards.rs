//! Tests for Innistrad Tier 7 cards (upkeep/end-step triggers + curses).

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::sba::check_state_based_actions_with_registry;
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
        state.move_object(c, Zone::Graveyard);
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
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    triggers::process_triggers(&mut state, &reg);

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

    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    triggers::process_triggers(&mut state, &reg);

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
        );
        state.get_object_mut(z).unwrap().summoning_sick = false;
    }

    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    triggers::process_triggers(&mut state, &reg);

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

    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::EndStep });
    triggers::process_triggers(&mut state, &reg);

    // SBAs to move destroyed creature to graveyard.
    check_state_based_actions_with_registry(&mut state, Some(&reg));

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

    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::EndStep });
    triggers::process_triggers(&mut state, &reg);

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
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse = state.create_object(curse_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(curse).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_object_mut(curse).unwrap().attached_to_player = Some(P1);

    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    triggers::process_triggers(&mut state, &reg);

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
    let curse_card_id = reg.get_id_by_name("Curse of Death's Hold").unwrap();
    let curse = state.create_object(curse_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(curse).unwrap().name = "Curse of Death's Hold".into();
    state.get_object_mut(curse).unwrap().attached_to_player = Some(P1);

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

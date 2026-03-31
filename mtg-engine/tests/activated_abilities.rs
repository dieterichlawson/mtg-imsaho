//! Tests for Innistrad Tier 10 cards with activated abilities:
//! Manor Skeleton, Kessig Wolf, Feral Ridgewolf, Darkthicket Wolf,
//! Lantern Spirit, Avacynian Priest.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ══════════════════════════════════════════════════════════════════
// Manor Skeleton — {1}{B} 1/1 Skeleton, Haste, {1}{B}: Regenerate
// ══════════════════════════════════════════════════════════════════

#[test]
fn manor_skeleton_has_correct_stats() {
    let reg = registry();
    let id = reg.get_id_by_name("Manor Skeleton").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(1));
    assert!(data.keywords.contains(&Keyword::Haste));
    assert!(data.subtypes.contains(&"Skeleton".to_string()));
}

#[test]
fn manor_skeleton_regenerate_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let skeleton = named_creature(&mut state, &reg, "Manor Skeleton", P0);

    // Add mana to pay {1}{B} for regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(activate.is_some(), "Should be able to activate regenerate ability");

    state = engine::submit_action(&state, activate.unwrap(), &reg);
    assert_eq!(state.get_object(skeleton).unwrap().regeneration_shields, 1,
        "Activating should add a regeneration shield");
}

#[test]
fn manor_skeleton_regeneration_saves_from_lethal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let skeleton = named_creature(&mut state, &reg, "Manor Skeleton", P0);

    // Activate regenerate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    // Deal lethal damage.
    state.get_object_mut(skeleton).unwrap().damage_marked = 1;
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Should survive via regeneration.
    assert_eq!(state.get_object(skeleton).unwrap().zone, Zone::Battlefield,
        "Manor Skeleton should survive via regeneration");
    assert_eq!(state.get_object(skeleton).unwrap().damage_marked, 0,
        "Damage should be cleared after regeneration");
}

// ══════════════════════════════════════════════════════════════════
// Kessig Wolf — {2}{R} 3/1 Wolf, {1}{R}: gains first strike EOT
// ══════════════════════════════════════════════════════════════════

#[test]
fn kessig_wolf_has_correct_stats() {
    let reg = registry();
    let id = reg.get_id_by_name("Kessig Wolf").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(3));
    assert_eq!(data.toughness, Some(1));
    assert!(data.subtypes.contains(&"Wolf".to_string()));
}

#[test]
fn kessig_wolf_gains_first_strike() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_creature(&mut state, &reg, "Kessig Wolf", P0);

    // Should not have first strike initially.
    assert!(!state.has_keyword(wolf, Keyword::FirstStrike, &reg),
        "Kessig Wolf should not have first strike initially");

    // Activate ability: pay {1}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    // Should now have first strike.
    assert!(state.has_keyword(wolf, Keyword::FirstStrike, &reg),
        "Kessig Wolf should have first strike after activation");
}

// ══════════════════════════════════════════════════════════════════
// Feral Ridgewolf — {2}{R} 1/2 Wolf, Trample, {1}{R}: +2/+0 EOT
// ══════════════════════════════════════════════════════════════════

#[test]
fn feral_ridgewolf_has_correct_stats() {
    let reg = registry();
    let id = reg.get_id_by_name("Feral Ridgewolf").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(2));
    assert!(data.keywords.contains(&Keyword::Trample));
    assert!(data.subtypes.contains(&"Wolf".to_string()));
}

#[test]
fn feral_ridgewolf_gets_plus_2_plus_0() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_creature(&mut state, &reg, "Feral Ridgewolf", P0);

    assert_eq!(state.effective_power(wolf, &reg), Some(1));
    assert_eq!(state.effective_toughness(wolf, &reg), Some(2));

    // Activate: pay {1}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    assert_eq!(state.effective_power(wolf, &reg), Some(3),
        "Feral Ridgewolf should be 3 power after +2/+0");
    assert_eq!(state.effective_toughness(wolf, &reg), Some(2),
        "Feral Ridgewolf toughness should be unchanged");
}

#[test]
fn feral_ridgewolf_can_activate_multiple_times() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_creature(&mut state, &reg, "Feral Ridgewolf", P0);

    // Activate twice.
    for _ in 0..2 {
        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
        state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
        let legal = engine::legal_actions(&state, &reg);
        let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
        state = engine::submit_action(&state, &activate, &reg);
    }

    assert_eq!(state.effective_power(wolf, &reg), Some(5),
        "Feral Ridgewolf should be 5 power after two activations (+2+2)");
}

// ══════════════════════════════════════════════════════════════════
// Darkthicket Wolf — {1}{G} 2/2 Wolf, {2}{G}: +2/+2 EOT (once/turn)
// ══════════════════════════════════════════════════════════════════

#[test]
fn darkthicket_wolf_has_correct_stats() {
    let reg = registry();
    let id = reg.get_id_by_name("Darkthicket Wolf").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(2));
    assert_eq!(data.toughness, Some(2));
    assert!(data.subtypes.contains(&"Wolf".to_string()));
}

#[test]
fn darkthicket_wolf_gets_plus_2_plus_2() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_creature(&mut state, &reg, "Darkthicket Wolf", P0);

    // Activate: pay {2}{G}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    assert_eq!(state.effective_power(wolf, &reg), Some(4),
        "Darkthicket Wolf should be 4/4 after +2/+2");
    assert_eq!(state.effective_toughness(wolf, &reg), Some(4));
}

#[test]
fn darkthicket_wolf_once_per_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_creature(&mut state, &reg, "Darkthicket Wolf", P0);

    // First activation should work.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    // Second activation should not be available.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    let legal = engine::legal_actions(&state, &reg);
    let has_activate = legal.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(!has_activate,
        "Darkthicket Wolf should not be able to activate a second time in the same turn");
}

// ══════════════════════════════════════════════════════════════════
// Lantern Spirit — {2}{U} 2/1 Spirit, Flying, {U}: return to hand
// ══════════════════════════════════════════════════════════════════

#[test]
fn lantern_spirit_has_correct_stats() {
    let reg = registry();
    let id = reg.get_id_by_name("Lantern Spirit").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(2));
    assert_eq!(data.toughness, Some(1));
    assert!(data.keywords.contains(&Keyword::Flying));
    assert!(data.subtypes.contains(&"Spirit".to_string()));
}

#[test]
fn lantern_spirit_returns_to_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spirit = named_creature(&mut state, &reg, "Lantern Spirit", P0);

    // Should be on battlefield.
    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Battlefield);

    // Activate: pay {U}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    // Should now be in hand.
    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "Lantern Spirit should be returned to hand after activation");
}

// ══════════════════════════════════════════════════════════════════
// Avacynian Priest — {1}{W} 1/2 Human Cleric, {1},{T}: Tap non-Human
// ══════════════════════════════════════════════════════════════════

#[test]
fn avacynian_priest_has_correct_stats() {
    let reg = registry();
    let id = reg.get_id_by_name("Avacynian Priest").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(2));
    assert!(data.subtypes.contains(&"Human".to_string()));
    assert!(data.subtypes.contains(&"Cleric".to_string()));
}

#[test]
fn avacynian_priest_taps_non_human_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_creature(&mut state, &reg, "Avacynian Priest", P0);

    // Create a non-Human creature (Kessig Wolf).
    let wolf = named_creature(&mut state, &reg, "Kessig Wolf", P1);
    assert!(!state.get_object(wolf).unwrap().tapped, "Wolf should start untapped");

    // Add mana {1} for the ability.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| {
        matches!(a, Action::ActivateAbility { targets, .. } if targets.contains(&Target::Object(wolf)))
    });
    assert!(activate.is_some(), "Should be able to target the non-Human wolf");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    assert!(state.get_object(wolf).unwrap().tapped,
        "Wolf should be tapped by Avacynian Priest");
    assert!(state.get_object(priest).unwrap().tapped,
        "Priest should be tapped as part of the cost");
}

#[test]
fn avacynian_priest_cannot_target_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_creature(&mut state, &reg, "Avacynian Priest", P0);

    // Create a Human creature (Elder Cathar).
    let human = named_creature(&mut state, &reg, "Elder Cathar", P1);

    // Add mana.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let targets_human = legal.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { targets, .. } if targets.contains(&Target::Object(human)))
    });
    assert!(!targets_human,
        "Avacynian Priest should not be able to target Human creatures");
}

#[test]
fn avacynian_priest_requires_tap() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_creature(&mut state, &reg, "Avacynian Priest", P0);

    // Create a non-Human target.
    let wolf = named_creature(&mut state, &reg, "Kessig Wolf", P1);

    // First activation should work.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { .. })).unwrap().clone();
    state = engine::submit_action(&state, &activate, &reg);

    // Priest is now tapped, should not be able to activate again.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    let legal = engine::legal_actions(&state, &reg);
    let has_activate = legal.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(!has_activate,
        "Tapped Avacynian Priest should not be able to activate again");
}

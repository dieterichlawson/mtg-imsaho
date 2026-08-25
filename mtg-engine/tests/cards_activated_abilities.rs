//! Creatures and lands whose behaviour is an activated ability (CR 602).
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Elder of Laurels
//! - Gavony Township
//! - Mindshrieker
//! - Nephalia Drownyard
//! - Skirsdag High Priest
//! - Stensia Bloodhall

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Elder of Laurels
// ══════════════════════════════════════════════════════════════════

#[test]
fn elder_of_laurels_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Elder of Laurels").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(2));
    assert_eq!(data.toughness, Some(3));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 3);
    assert!(data.subtypes.contains(&"Human".to_string()));
    assert!(data.subtypes.contains(&"Advisor".to_string()));
}

/// Elder of Laurels gives +X/+X where X = number of creatures you control.
#[test]
fn elder_of_laurels_pumps_by_creature_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_creature(&mut state, &reg, "Elder of Laurels", P0);
    let target = ready_creature(&mut state, P0, 2, 2);
    let _extra = ready_creature(&mut state, P0, 1, 1);
    // P0 controls 3 creatures: elder, target, extra. So X = 3.

    // Give P0 mana to activate: {3}{G}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let legal = engine::legal_actions(&state, &reg);
    // Find activate ability action targeting the 2/2.
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == elder && targets.contains(&Target::Object(target))
    ));
    assert!(activate.is_some(), "Should be able to activate Elder of Laurels targeting the 2/2");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // Target should get +3/+3 (3 creatures controlled).
    assert_eq!(state.effective_power(target, &reg), Some(5));
    assert_eq!(state.effective_toughness(target, &reg), Some(5));
}

// ══════════════════════════════════════════════════════════════════
// Mindshrieker
// ══════════════════════════════════════════════════════════════════

#[test]
fn mindshrieker_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Mindshrieker").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(1));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
    assert!(data.keywords.contains(&Keyword::Flying));
}

/// Mindshrieker mills a card and gets +X/+X where X is the milled card's mana value.
#[test]
fn mindshrieker_mills_and_pumps() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shrieker = named_creature(&mut state, &reg, "Mindshrieker", P0);

    // Put a known card on top of P1's library: Kindercatch (mana value 6).
    let kindercatch_id = reg.get_id_by_name("Kindercatch").unwrap();
    let lib_card = state.create_object(kindercatch_id, P1, Zone::Library, Some(6), Some(6));
    state.get_object_mut(lib_card).unwrap().name = "Kindercatch".into();
    state.players[1].library_order = vec![lib_card];

    // Give P0 mana to activate: {2}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == shrieker && targets.contains(&Target::Player(P1))
    ));
    assert!(activate.is_some(), "Should be able to activate Mindshrieker targeting P1");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // Card should be milled to graveyard.
    assert_eq!(state.get_object(lib_card).unwrap().zone, Zone::Graveyard);

    // Mindshrieker should get +6/+6 (Kindercatch costs 6).
    assert_eq!(state.effective_power(shrieker, &reg), Some(7));
    assert_eq!(state.effective_toughness(shrieker, &reg), Some(7));
}

/// Mindshrieker with a land (mana value 0) gets +0/+0.
#[test]
fn mindshrieker_mills_land_no_pump() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shrieker = named_creature(&mut state, &reg, "Mindshrieker", P0);

    // Put a Forest on top of P1's library (mana value 0).
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let lib_card = state.create_object(forest_id, P1, Zone::Library, None, None);
    state.players[1].library_order = vec![lib_card];

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == shrieker && targets.contains(&Target::Player(P1))
    ));
    assert!(activate.is_some());

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // No pump (lands have no mana cost, so mana value = 0).
    assert_eq!(state.effective_power(shrieker, &reg), Some(1));
    assert_eq!(state.effective_toughness(shrieker, &reg), Some(1));
}

// ══════════════════════════════════════════════════════════════════
// Skirsdag High Priest
// ══════════════════════════════════════════════════════════════════

#[test]
fn skirsdag_high_priest_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Skirsdag High Priest").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(2));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
    assert!(data.subtypes.contains(&"Human".to_string()));
    assert!(data.subtypes.contains(&"Cleric".to_string()));
}

/// Skirsdag High Priest creates a 5/5 Demon with flying when morbid is active.
#[test]
fn skirsdag_high_priest_creates_demon_with_morbid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let priest = named_creature(&mut state, &reg, "Skirsdag High Priest", P0);
    let _helper1 = ready_creature(&mut state, P0, 1, 1);
    let _helper2 = ready_creature(&mut state, P0, 1, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == priest
    ));
    assert!(activate.is_some(), "Should be able to activate Skirsdag High Priest with morbid + 2 untapped creatures");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // Should have created a 5/5 Demon token with flying.
    let demons: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Demon" && o.power == Some(5) && o.toughness == Some(5))
        .collect();
    assert_eq!(demons.len(), 1, "Should have one 5/5 Demon token");
    assert!(demons[0].keywords.contains(&Keyword::Flying), "Demon should have flying");
}

/// Skirsdag High Priest can't activate without morbid.
#[test]
fn skirsdag_high_priest_no_morbid() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // creature_died_this_turn is false by default.

    let priest = named_creature(&mut state, &reg, "Skirsdag High Priest", P0);
    let _helper1 = ready_creature(&mut state, P0, 1, 1);
    let _helper2 = ready_creature(&mut state, P0, 1, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == priest
    ));
    assert!(activate.is_none(), "Should not be able to activate without morbid");
}

/// Skirsdag High Priest needs 2 other untapped creatures.
#[test]
fn skirsdag_high_priest_needs_two_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let priest = named_creature(&mut state, &reg, "Skirsdag High Priest", P0);
    let _helper1 = ready_creature(&mut state, P0, 1, 1);
    // Only 1 other untapped creature.

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == priest
    ));
    assert!(activate.is_none(), "Should not be able to activate with only 1 other untapped creature");
}

// ══════════════════════════════════════════════════════════════════
// Gavony Township
// ══════════════════════════════════════════════════════════════════

#[test]
fn gavony_township_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Gavony Township").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Land));
    assert_eq!(data.cost, None);
}

/// Gavony Township puts a +1/+1 counter on each creature you control.
#[test]
fn gavony_township_counters_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let township_card_id = reg.get_id_by_name("Gavony Township").unwrap();
    let township = state.create_object(township_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(township).unwrap().name = "Gavony Township".into();
    state.get_object_mut(township).unwrap().summoning_sick = false;

    let c1 = ready_creature(&mut state, P0, 2, 2);
    let c2 = ready_creature(&mut state, P0, 3, 3);
    let enemy = ready_creature(&mut state, P1, 4, 4); // Opponent's creature: should not get counter.

    // Give mana: {2}{G}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == township
    ));
    assert!(activate.is_some(), "Should be able to activate Gavony Township");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // Both of P0's creatures should have +1/+1 counters.
    assert_eq!(counters_of(&state, c1, CounterType::PlusOnePlusOne), 1);
    assert_eq!(counters_of(&state, c2, CounterType::PlusOnePlusOne), 1);
    // Opponent's creature should NOT have a counter.
    assert_eq!(counters_of(&state, enemy, CounterType::PlusOnePlusOne), 0);
}

// ══════════════════════════════════════════════════════════════════
// Nephalia Drownyard
// ══════════════════════════════════════════════════════════════════

#[test]
fn nephalia_drownyard_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Nephalia Drownyard").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Land));
    assert_eq!(data.cost, None);
}

/// Nephalia Drownyard mills 3 cards from target player.
#[test]
fn nephalia_drownyard_mills_three() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let drownyard_card_id = reg.get_id_by_name("Nephalia Drownyard").unwrap();
    let drownyard = state.create_object(drownyard_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(drownyard).unwrap().name = "Nephalia Drownyard".into();
    state.get_object_mut(drownyard).unwrap().summoning_sick = false;

    // Put 5 cards in P1's library.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut lib = Vec::new();
    for _ in 0..5 {
        let id = state.create_object(forest_id, P1, Zone::Library, None, None);
        lib.push(id);
    }
    state.players[1].library_order = lib.clone();

    // Give mana: {1}{U}{B}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == drownyard && targets.contains(&Target::Player(P1))
    ));
    assert!(activate.is_some(), "Should be able to activate Nephalia Drownyard targeting P1");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // P1 should have 2 cards left in library (5 - 3 = 2).
    assert_eq!(state.players[1].library_order.len(), 2);
    // 3 cards should be in the graveyard.
    let graveyard_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert_eq!(graveyard_count, 3);
}

// ══════════════════════════════════════════════════════════════════
// Stensia Bloodhall
// ══════════════════════════════════════════════════════════════════

#[test]
fn stensia_bloodhall_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Stensia Bloodhall").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Land));
    assert_eq!(data.cost, None);
}

/// Stensia Bloodhall deals 2 damage to target player.
#[test]
fn stensia_bloodhall_deals_2_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bloodhall_card_id = reg.get_id_by_name("Stensia Bloodhall").unwrap();
    let bloodhall = state.create_object(bloodhall_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(bloodhall).unwrap().name = "Stensia Bloodhall".into();
    state.get_object_mut(bloodhall).unwrap().summoning_sick = false;

    // Give mana: {3}{B}{R}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == bloodhall && targets.contains(&Target::Player(P1))
    ));
    assert!(activate.is_some(), "Should be able to activate Stensia Bloodhall targeting P1");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    assert_eq!(state.get_player(P1).life, 18, "P1 should take 2 damage (20 - 2 = 18)");
}

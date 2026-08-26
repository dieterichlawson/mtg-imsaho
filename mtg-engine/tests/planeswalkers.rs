//! Planeswalkers as a card type: the loyalty state-based action (CR 704.5i)
//! and loyalty-ability activation (CR 606).
//!
//! These are engine rules, not card behaviour. They used Liliana of the Veil
//! as a fixture and lived in `cards_complex_creatures.rs` among tests about
//! what particular creatures do; the card is incidental here — any
//! planeswalker would serve.

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

#[test]
fn planeswalker_with_zero_loyalty_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    // 0 loyalty counters.

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Graveyard);
}

#[test]
fn planeswalker_with_loyalty_survives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.add_counters(liliana, CounterType::Loyalty, 3);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Battlefield);
}

#[test]
fn loyalty_abilities_appear_in_legal_actions() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.add_counters(liliana, CounterType::Loyalty, 3);

    let legal = engine::legal_actions(&state, &reg);

    // Should have loyalty ability actions.
    let loyalty_actions: Vec<_> = legal.actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { .. }))
        .collect();
    // +1 (no target) + -2 targeting P0 + -2 targeting P1 = at least 3 actions.
    // (Not -6, since loyalty is only 3.)
    assert!(loyalty_actions.len() >= 3, "Expected at least 3 loyalty actions (+1 untargeted, -2 x2 player targets), got {}", loyalty_actions.len());

    // Verify +1 has no targets and -2 has player targets.
    let plus_one: Vec<_> = loyalty_actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { ability_index: 0, .. }))
        .collect();
    assert_eq!(plus_one.len(), 1, "+1 should have exactly one action (untargeted)");

    let minus_two: Vec<_> = loyalty_actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { ability_index: 1, .. }))
        .collect();
    assert_eq!(minus_two.len(), 2, "-2 should have two actions (targeting P0 and P1)");
}

#[test]
fn loyalty_ability_adjusts_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.add_counters(liliana, CounterType::Loyalty, 3);

    // Give both players cards to discard.
    let _p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let _p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    // Activate +1.
    let new_state = engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana,
        ability_index: 0,
        targets: vec![],
    }, &reg);

    // Loyalty should be 4 (3 + 1).
    assert_eq!(new_state.get_counter_count(liliana, CounterType::Loyalty), 4);

}

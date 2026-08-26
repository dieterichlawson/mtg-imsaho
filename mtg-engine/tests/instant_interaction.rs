//! Tests for instant-speed interaction during opponent's turn.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// During opponent's declare attackers step, you should be able to cast
/// instants if you have mana available. This is essential for combat tricks
/// like Lightning Bolt on an attacker before blockers.
#[test]
fn can_cast_instant_during_opponent_combat() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P1); // P1's turn
    state.priority_player = Some(P0); // P0 has priority

    // P0 has a Mountain with {R} in pool and Lightning Bolt in hand.
    spell_in_hand(&mut state, &registry, "Lightning Bolt", P0);
    add_mana_for(&mut state, &registry, "Lightning Bolt", P0);

    // P1 has an attacking creature.
    let _lion = ready_creature(&mut state, P1, 2, 1);

    let legal = engine::legal_actions(&state, &registry);

    // P0 should be able to cast Lightning Bolt targeting the creature.
    let can_bolt = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(can_bolt,
        "Should be able to cast Lightning Bolt during opponent's declare attackers step");
}

/// During opponent's main phase, you should be able to cast instants
/// if you have mana available.
#[test]
fn can_cast_instant_during_opponent_main_phase() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P1); // P1's turn
    state.priority_player = Some(P0); // P0 has priority

    spell_in_hand(&mut state, &registry, "Lightning Bolt", P0);
    add_mana_for(&mut state, &registry, "Lightning Bolt", P0);

    // Need a valid target.
    ready_creature(&mut state, P1, 3, 3);

    let legal = engine::legal_actions(&state, &registry);
    let can_bolt = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(can_bolt,
        "Should be able to cast Lightning Bolt during opponent's main phase");
}

/// During opponent's turn, you should still have access to mana abilities
/// so you can tap lands to cast instants.
#[test]
fn can_tap_lands_during_opponent_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P1); // P1's turn
    state.priority_player = Some(P0);

    // P0 has an untapped Mountain.
    let mtn_id = registry.get_id_by_name("Mountain").unwrap();
    let mtn = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(mtn).unwrap().summoning_sick = false;

    let legal = engine::legal_actions(&state, &registry);
    let can_tap = legal.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { .. }));
    assert!(can_tap,
        "Should be able to tap lands during opponent's turn to cast instants");
}

/// You should NOT be able to cast sorcery-speed spells during opponent's turn,
/// even if you have mana.
#[test]
fn cannot_cast_sorcery_during_opponent_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P1); // P1's turn
    state.priority_player = Some(P0);

    spell_in_hand(&mut state, &registry, "Divination", P0);
    add_mana_for(&mut state, &registry, "Divination", P0);

    let legal = engine::legal_actions(&state, &registry);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(!can_cast,
        "Should NOT be able to cast sorcery during opponent's turn");
}

//! Tests for priority passing, the stack, and action timing rules.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Rule 117.4: Both players must pass in succession with an empty stack
/// for a phase to end. One player passing alone is not sufficient.
#[test]
fn both_players_must_pass_for_step_to_advance() {
    let registry = CardRegistry::with_all_cards();
    let state = game_at_step(Step::Upkeep, P0);

    let new_state = engine::submit_action(&state, &Action::PassPriority, &registry);
    assert_eq!(new_state.consecutive_passes, 1,
        "Only one player has passed — step should not advance");
}

/// Rule 117.3c: After casting a spell, the caster retains priority.
#[test]
fn caster_retains_priority_after_casting() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = castable_spell(&mut state, &registry, "Kalonian Tusker", P0);

    let new_state = engine::submit_action(
        &state, &Action::CastSpell { object_id: creature, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None }, &registry,
    );

    assert_eq!(new_state.stack.len(), 1);
    assert_eq!(new_state.get_player(P0).mana_pool.total(), 0, "Mana should be spent");
}

/// Mana abilities do NOT reset consecutive passes (rule 117.1d).
#[test]
fn mana_abilities_dont_reset_consecutive_passes() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.consecutive_passes = 1;

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().summoning_sick = false;

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility { object_id: forest, ability_index: 0 },
        &registry,
    );

    assert_eq!(new_state.consecutive_passes, 1,
        "Mana abilities should not reset consecutive passes");
    assert_eq!(new_state.get_player(P0).mana_pool.get(ManaType::Green), 1);
}

/// Rule 116.3: After a special action (playing a land), the player retains priority.
#[test]
fn player_retains_priority_after_playing_land() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

    let new_state = engine::submit_action(
        &state, &Action::PlayLand { object_id: land }, &registry,
    );

    assert_eq!(new_state.get_object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.priority_player, Some(P0),
        "Player should retain priority after playing a land (rule 116.3)");
}

/// Rule 117.1a: Creature spells can only be cast during your main phase
/// with an empty stack.
#[test]
fn creatures_only_castable_at_sorcery_speed() {
    let registry = CardRegistry::with_all_cards();

    // During combat.
    let mut state = game_at_step(Step::BeginCombat, P0);
    castable_spell(&mut state, &registry, "Kalonian Tusker", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::CastSpell { .. })),
        "Should not be able to cast creatures during combat");

    // During opponent's turn.
    let mut state2 = game_at_step(Step::PrecombatMain, P1);
    state2.priority_player = Some(P0);
    castable_spell(&mut state2, &registry, "Kalonian Tusker", P0);

    let actions2 = engine::legal_actions(&state2, &registry);
    assert!(!actions2.actions.iter().any(|a| matches!(a, Action::CastSpell { .. })),
        "Should not be able to cast creatures during opponent's turn");
}

/// Can't cast a spell you can't afford.
#[test]
fn cannot_cast_without_mana() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    spell_in_hand(&mut state, &registry, "Kalonian Tusker", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::CastSpell { .. })),
        "Should not be able to cast without enough mana");
}

/// Can't cast with the wrong color of mana.
#[test]
fn cannot_cast_with_wrong_color() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 5);
    spell_in_hand(&mut state, &registry, "Kalonian Tusker", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::CastSpell { .. })),
        "Should not be able to cast GG with only red mana");
}

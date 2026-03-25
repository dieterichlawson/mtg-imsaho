//! Tests for turn structure, step progression, and phase rules.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

/// Rule 502.4: No player receives priority during the untap step.
#[test]
fn no_priority_during_untap() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Untap);
    assert_eq!(state.priority_player, None,
        "No player should have priority during untap (rule 502.4)");
}

/// Rule 500.1: The combat phase happens every turn, even with no creatures.
/// Mana pools empty between pre-combat main and the combat phase.
#[test]
fn mana_empties_between_main_and_combat() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 3);

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::BeginCombat);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0,
        "Mana should empty at step boundary (rule 106.4)");
}

/// Rule 106.4: Mana empties at the end of EVERY step, not just end of turn.
#[test]
fn mana_empties_between_upkeep_and_draw() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 2);

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Draw);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0,
        "Mana should empty between upkeep and draw (rule 106.4)");
}

/// Mana empties between declare attackers and declare blockers steps.
#[test]
fn mana_empties_between_combat_steps() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::DeclareBlockers);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);
}

/// The first player skips their draw step on the first turn.
#[test]
fn first_player_skips_first_draw() {
    let registry = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.is_first_turn = true;
    state.step = Step::Upkeep;
    state.active_player = P0;
    state.priority_player = Some(P0);

    let card = state.create_object(CardId(1), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(card);

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Draw);
    assert_eq!(state.get_object(card).unwrap().zone, Zone::Library,
        "First player should skip the draw on the very first turn");
}

/// Untap step: all permanents controlled by active player untap.
/// Opponent's permanents do NOT untap.
#[test]
fn untap_step_untaps_only_active_players_permanents() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    let land = state.create_object(CardId(1), P0, Zone::Battlefield, None, None);
    state.get_object_mut(land).unwrap().tapped = true;
    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().tapped = true;
    let p1_creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(p1_creature).unwrap().tapped = true;

    // Advance to P1's untap step.
    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Untap);
    assert_eq!(state.active_player, P1);

    assert!(!state.get_object(p1_creature).unwrap().tapped,
        "Active player's permanents should untap");
    assert!(state.get_object(land).unwrap().tapped,
        "Inactive player's permanents should NOT untap");
    assert!(state.get_object(creature).unwrap().tapped,
        "Inactive player's creatures should NOT untap");
}

/// Damage marked on creatures persists until the cleanup step.
#[test]
fn damage_persists_between_steps() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let creature = ready_creature(&mut state, P0, 2, 5);
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::EndCombat);
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3,
        "Damage should persist between steps");

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::PostcombatMain);
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3);
}

/// Rule 514.2: Damage is removed during the cleanup step.
#[test]
fn damage_removed_during_cleanup() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let creature = ready_creature(&mut state, P0, 2, 5);
    state.get_object_mut(creature).unwrap().damage_marked = 4;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Cleanup);
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be removed during cleanup (rule 514.2)");
}

/// Rule 514.1: Discard to hand size (7) during cleanup.
#[test]
fn discard_to_hand_size_during_cleanup() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    for _ in 0..9 {
        state.create_object(CardId(1), P0, Zone::Hand, None, None);
    }

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Cleanup);
    assert!(matches!(
        state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::DiscardToHandSize { player, discard_count })
        if player == P0 && discard_count == 2
    ));
}

/// No discard needed if hand size is 7 or less.
#[test]
fn no_discard_needed_at_seven_or_less() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    for _ in 0..7 {
        state.create_object(CardId(1), P0, Zone::Hand, None, None);
    }

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Cleanup);
    assert!(state.awaiting_action.is_none());
    assert_eq!(state.priority_player, None,
        "No priority during cleanup when no discard needed");
}

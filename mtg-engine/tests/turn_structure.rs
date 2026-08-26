//! Tests for turn structure, step progression, and phase rules.

mod common;

use common::*;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

/// Rule 502.4: No player receives priority during the untap step.
#[test]
fn no_priority_during_untap() {
    let registry = registry();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Untap);
    assert_eq!(state.priority_player, None,
        "No player should have priority during untap (rule 502.4)");
}

/// CR 106.4: a mana pool empties at the end of every step and phase, not only
/// at end of turn. Three boundaries, because "it emptied" at one of them is
/// also true of an engine that empties pools at some unrelated moment.
#[test]
fn a_mana_pool_empties_at_every_step_boundary() {
    // (step to float mana in, the step that follows it)
    const BOUNDARIES: &[(Step, Step)] = &[
        (Step::Upkeep, Step::Draw),
        (Step::PrecombatMain, Step::BeginCombat),
        (Step::DeclareAttackers, Step::DeclareBlockers),
    ];

    for &(from, to) in BOUNDARIES {
        let registry = registry();
        let mut state = game_at_step(from, P0);
        state.get_player_mut(P0).mana_pool.add(ManaType::Green, 3);

        engine::advance_step(&mut state, &registry);

        assert_eq!(state.step, to, "test setup: {from:?} is followed by {to:?}");
        assert_eq!(state.get_player(P0).mana_pool.total(), 0,
            "mana floated in {from:?} is gone by {to:?}");
    }
}

/// The first player skips their draw step on the first turn.
#[test]
fn first_player_skips_first_draw() {
    let registry = registry();
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
    let registry = registry();
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
    let registry = registry();
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
    let registry = registry();
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
    let registry = registry();
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

/// Submits `Action::DiscardCards` through `engine::submit_action` during
/// the cleanup discard step. Exercises the hand-size-cleanup branch of
/// the handler (is_hand_size=true), including the aggregated log message.
#[test]
fn submit_action_discard_cards_for_cleanup_moves_cards_to_graveyard() {
    use mtg_engine::actions::Action;
    use mtg_engine::state::AwaitingAction;

    let registry = registry();
    let mut state = game_at_step(Step::Cleanup, P0);
    let mut hand = Vec::new();
    for _ in 0..9 {
        hand.push(state.create_object(CardId(1), P0, Zone::Hand, None, None));
    }
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });

    let to_discard = vec![hand[0], hand[1]];
    let new_state = engine::submit_action(
        &state,
        &Action::DiscardCards { cards: to_discard.clone() },
        &registry,
    );

    assert!(new_state.awaiting_action.is_none());
    for id in &to_discard {
        assert_eq!(new_state.get_object(*id).unwrap().zone, Zone::Graveyard,
            "discarded card should be in graveyard");
    }
    let remaining_hand = new_state.objects_in_zone(Zone::Hand, P0).len();
    assert_eq!(remaining_hand, 7, "hand size should be 7 after discarding 2 of 9");
    assert!(new_state.game_log.iter().any(|e| e.message.contains("cleanup")),
        "log should annotate the discard as cleanup-driven");
}

/// Submits `Action::DiscardCards` without a `DiscardToHandSize`
/// awaiting-action (e.g. from a spell like Lay Bare or an activated
/// ability that discards). Exercises the non-cleanup branch and the
/// per-card log messages.
#[test]
fn submit_action_discard_cards_without_awaiting_logs_per_card() {
    use mtg_engine::actions::Action;

    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c1 = state.create_object(CardId(1), P0, Zone::Hand, None, None);
    let c2 = state.create_object(CardId(1), P0, Zone::Hand, None, None);

    let new_state = engine::submit_action(
        &state,
        &Action::DiscardCards { cards: vec![c1, c2] },
        &registry,
    );

    assert_eq!(new_state.get_object(c1).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(c2).unwrap().zone, Zone::Graveyard);
    let discard_msgs = new_state.game_log.iter()
        .filter(|e| e.message.contains("discarded"))
        .count();
    assert!(discard_msgs >= 2,
        "non-cleanup path should log one 'discarded' message per card (got {discard_msgs})");
}

/// No discard needed if hand size is 7 or less.
#[test]
fn no_discard_needed_at_seven_or_less() {
    let registry = registry();
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

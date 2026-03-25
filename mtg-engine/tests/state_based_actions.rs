//! Tests for state-based actions (rule 704).

mod common;

use common::*;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::GameResult;
use mtg_engine::types::*;

/// Rule 104.4a: If both players reach 0 life simultaneously, it's a draw.
#[test]
fn simultaneous_life_loss_is_draw() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = 0;
    state.players[1].life = -2;

    check_state_based_actions(&mut state);

    assert!(state.players[0].lost);
    assert!(state.players[1].lost);
    assert_eq!(state.result, Some(GameResult::Draw),
        "Both players at <=0 life simultaneously should be a draw (rule 104.4a)");
}

/// Rule 704.5g: Creature with damage >= toughness dies, but only when SBAs
/// are checked — not at the instant damage is dealt.
#[test]
fn creature_survives_until_sba_check() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let creature = ready_creature(&mut state, P0, 2, 3);
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    // Before SBA check, creature is still on the battlefield.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Rule 704.5b: Drawing from an empty library sets a flag. Player loses
/// when SBAs are next checked, not immediately.
#[test]
fn empty_library_loss_is_deferred_to_sba() {
    let mut state = game_at_step(Step::Draw, P0);

    engine::draw_cards(&mut state, P0, 1);

    assert!(state.get_player(P0).has_drawn_from_empty);
    assert!(!state.get_player(P0).lost,
        "Player should not lose immediately on empty draw (rule 704.5b)");

    check_state_based_actions(&mut state);
    assert!(state.get_player(P0).lost);
}

/// Rule 704.5f: Creature with 0 toughness goes to graveyard.
#[test]
fn zero_toughness_creature_dies() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Battlefield, Some(5), Some(0));

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Negative toughness also kills a creature.
#[test]
fn negative_toughness_creature_dies() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Battlefield, Some(3), Some(-1));

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Rule 704.3: SBAs repeat until none are performed. Multiple SBAs
/// can happen in one check cycle.
#[test]
fn sbas_repeat_until_stable() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = 0;
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    check_state_based_actions(&mut state);

    assert!(state.players[0].lost);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Creature with damage less than its toughness survives.
#[test]
fn creature_survives_non_lethal_damage() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 4);
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "3 damage on a 4-toughness creature should not be lethal");
}

/// Exactly lethal damage kills a creature.
#[test]
fn exactly_lethal_damage_kills() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 3);
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Overkill damage also kills.
#[test]
fn overkill_damage_kills() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature).unwrap().damage_marked = 100;

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Player at exactly 0 life loses.
#[test]
fn exactly_zero_life_is_loss() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = 0;

    check_state_based_actions(&mut state);
    assert!(state.players[0].lost);
}

/// Negative life also loses.
#[test]
fn negative_life_is_loss() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = -10;

    check_state_based_actions(&mut state);
    assert!(state.players[0].lost);
}

/// Player at 1 life does NOT lose.
#[test]
fn one_life_is_not_loss() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = 1;

    check_state_based_actions(&mut state);
    assert!(!state.players[0].lost);
}

/// No SBAs when everything is fine — check returns false.
#[test]
fn no_sbas_when_stable() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P0, 3, 3);

    assert!(!check_state_based_actions(&mut state));
}

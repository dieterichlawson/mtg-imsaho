//! Tests for land plays and mana payment rules.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::mana;
use mtg_engine::types::*;

/// Rule 305.1: You can play a land during your second main phase.
#[test]
fn can_play_land_in_postcombat_main() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    state.create_object(forest_id, P0, Zone::Hand, None, None);

    let actions = engine::legal_actions(&state, &registry);
    assert!(actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should be able to play land in postcombat main (rule 305.1)");
}

/// One land per turn: after playing one, you can't play another.
#[test]
fn only_one_land_per_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let land1 = state.create_object(forest_id, P0, Zone::Hand, None, None);
    state.create_object(forest_id, P0, Zone::Hand, None, None);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land1 }, &registry);

    assert_eq!(state.get_player(P0).land_plays_remaining, 0);
    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play a second land");
}

/// Land plays reset at the start of your turn (during untap).
#[test]
fn land_plays_reset_at_untap() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;
    state.get_player_mut(P0).land_plays_remaining = 0;

    loop {
        engine::advance_step(&mut state, &registry);
        if state.step == Step::Untap && state.active_player == P0 {
            break;
        }
    }

    assert_eq!(state.get_player(P0).land_plays_remaining, 1);
}

/// Rule 116.2a: Playing a land doesn't use the stack.
#[test]
fn playing_land_doesnt_use_stack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let land = state.create_object(forest_id, P0, Zone::Hand, None, None);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land }, &registry);

    assert!(state.stack.is_empty());
    assert_eq!(state.get_object(land).unwrap().zone, Zone::Battlefield);
}

/// A just-played land can be tapped for mana immediately.
#[test]
fn can_tap_just_played_land() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let land = state.create_object(forest_id, P0, Zone::Hand, None, None);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land }, &registry);

    let actions = engine::legal_actions(&state, &registry);
    let has_mana_ability = actions.actions.iter().any(|a| match a {
        Action::ActivateManaAbility { object_id, .. } => *object_id == land,
        _ => false,
    });
    assert!(has_mana_ability, "Should be able to tap a just-played land for mana");
}

/// Can't play a land during opponent's turn.
#[test]
fn cannot_play_land_during_opponent_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P1);
    state.priority_player = Some(P0); // P0 has priority but it's P1's turn

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    state.create_object(forest_id, P0, Zone::Hand, None, None);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play land during opponent's turn");
}

/// Can't play a land during combat.
#[test]
fn cannot_play_land_during_combat() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::BeginCombat, P0);

    let forest_id = registry.get_id_by_name("Forest").unwrap();
    state.create_object(forest_id, P0, Zone::Hand, None, None);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play land during combat");
}

/// Generic mana can be paid with any color.
#[test]
fn generic_mana_payable_with_any_color() {
    let mut pool = ManaPool::new();
    pool.add(ManaType::Blue, 3);

    // {2}{R} — can't pay the {R}
    let cost_needs_red = ManaCost::new(vec![
        ManaSymbol::Generic(2),
        ManaSymbol::Colored(Color::Red),
    ]);
    assert!(!mana::can_pay(&pool, &cost_needs_red));

    // {3} — all generic, payable with blue
    let cost_all_generic = ManaCost::new(vec![ManaSymbol::Generic(3)]);
    assert!(mana::can_pay(&pool, &cost_all_generic));
}

/// Mana payment deducts correctly and leaves leftover.
#[test]
fn mana_payment_leaves_correct_remainder() {
    let mut pool = ManaPool::new();
    pool.add(ManaType::Green, 3);
    pool.add(ManaType::Red, 1);

    let cost = ManaCost::new(vec![
        ManaSymbol::Generic(1),
        ManaSymbol::Colored(Color::Green),
    ]);

    mana::auto_pay(&mut pool, &cost).unwrap();

    assert_eq!(pool.total(), 2);
    // Auto-pay uses red for generic (it comes before green in the pay order),
    // so we should have 2 green left and 0 red.
    assert_eq!(pool.get(ManaType::Green), 2);
    assert_eq!(pool.get(ManaType::Red), 0);
}

/// Free cost (no mana required) can always be paid.
#[test]
fn free_cost_always_payable() {
    let pool = ManaPool::new();
    let cost = ManaCost::free();
    assert!(mana::can_pay(&pool, &cost));
}

/// Can't pay cost with empty pool.
#[test]
fn empty_pool_cannot_pay() {
    let pool = ManaPool::new();
    let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
    assert!(!mana::can_pay(&pool, &cost));
}

//! Tests for land plays and mana payment rules.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::mana;
use mtg_engine::types::*;

/// Rule 305.1: You can play a land during your second main phase.
#[test]
fn can_play_land_in_postcombat_main() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    spell_in_hand(&mut state, &registry, "Forest", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should be able to play land in postcombat main (rule 305.1)");
}

/// One land per turn: after playing one, you can't play another.
#[test]
fn only_one_land_per_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land1 = spell_in_hand(&mut state, &registry, "Forest", P0);
    spell_in_hand(&mut state, &registry, "Forest", P0);

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

    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land }, &registry);

    assert!(state.stack.is_empty());
    assert_eq!(state.get_object(land).unwrap().zone, Zone::Battlefield);
}

/// A just-played land can be tapped for mana immediately.
#[test]
fn can_tap_just_played_land() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

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

    spell_in_hand(&mut state, &registry, "Forest", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play land during opponent's turn");
}

/// Can't play a land during combat.
#[test]
fn cannot_play_land_during_combat() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::BeginCombat, P0);

    spell_in_hand(&mut state, &registry, "Forest", P0);

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

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Ghost Quarter doesn't shuffle the library after the land search.
/// Oracle: "put it onto the battlefield, then shuffle."
/// The code finds and places the land but never calls `library_order.shuffle()`.
/// We verify by checking the library has NO basic lands removed (search happens)
/// but the remaining order is unchanged (no shuffle).
#[test]
fn bug_ghost_quarter_missing_shuffle() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Ghost Quarter for P0
    let gq = named_creature(&mut state, &registry, "Ghost Quarter", P0);

    // Place a target land for P1
    let target_land = {
        let card_id = registry.get_id_by_name("Forest").unwrap();
        let id = state.create_object(card_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        id
    };

    // Put a mix of basic lands and non-lands in P1's library
    // Use different basic land types so we can track order
    let names = ["Plains", "Island", "Swamp", "Mountain", "Forest",
                 "Plains", "Island", "Swamp", "Mountain", "Forest"];
    for name in &names {
        let card_id = registry.get_id_by_name(name).unwrap();
        let id = state.create_object(card_id, P1, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = (*name).into();
        state.get_player_mut(P1).library_order.push(id);
    }

    // Activate Ghost Quarter's ability
    let behavior = registry.get(state.get_object(gq).unwrap().card_id).unwrap();
    state.move_object(gq, Zone::Graveyard, &registry);
    behavior.on_activate_ability(&mut state, gq, 1, &[Target::Object(target_land)], &registry);

    // Ghost Quarter now presents a "may search" choice. Resolve by choosing the first Plains.
    assert!(state.awaiting_action.is_some(), "Should present 'may search' choice");
    let first_plains = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options.first().cloned(),
        _ => None,
    };
    assert!(first_plains.is_some(), "Should have a Plains option");
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(first_plains),
        },
        &registry,
    );

    // After search: one Plains was removed from library and put on battlefield.
    let lib_after: Vec<_> = state.get_player(P1).library_order.clone();
    assert_eq!(lib_after.len(), 9, "One land should have been found and placed");

    // Library should be shuffled per oracle text.
    let names_after: Vec<String> = lib_after.iter()
        .filter_map(|id| state.get_object(*id).map(|o| o.name.clone()))
        .collect();
    let expected = vec!["Island", "Swamp", "Mountain", "Forest", "Plains", "Island", "Swamp", "Mountain", "Forest"];
    assert_ne!(names_after, expected,
        "Library should be shuffled after Ghost Quarter search, but order is preserved (no shuffle)");
}

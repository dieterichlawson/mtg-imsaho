//! Tests for Memory's Journey.
//!
//! Oracle: {1}{U} Instant
//! Target player shuffles up to three target cards from their graveyard into their library.
//! Flashback {G}

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::types::*;
/// Shuffles a card from your own graveyard into your library.
#[test]
fn shuffles_own_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(card, Zone::Graveyard, &reg);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P0), Target::Object(card)]);

    assert_eq!(new_state.get_object(card).unwrap().zone, Zone::Library,
        "Card should be shuffled into library");
}

/// Shuffles a card from opponent's graveyard into their library.
#[test]
fn shuffles_opponent_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(card, Zone::Graveyard, &reg);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P1), Target::Object(card)]);

    assert_eq!(new_state.get_object(card).unwrap().zone, Zone::Library,
        "Opponent's card should be shuffled into their library");
}

/// Can target up to 3 cards from the same graveyard.
#[test]
fn shuffles_up_to_three_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(card1, Zone::Graveyard, &reg);
    let card2 = spell_in_hand(&mut state, &reg, "Doom Blade", P0);
    state.move_object(card2, Zone::Graveyard, &reg);
    let card3 = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(card3, Zone::Graveyard, &reg);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![
        Target::Player(P0),
        Target::Object(card1),
        Target::Object(card2),
        Target::Object(card3),
    ]);

    assert_eq!(new_state.get_object(card1).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(card2).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(card3).unwrap().zone, Zone::Library);
}

/// Memory's Journey requires a player target (for choosing whose graveyard).
#[test]
fn legal_actions_dont_mix_graveyards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(own_card, Zone::Graveyard, &reg);
    let opp_card = spell_in_hand(&mut state, &reg, "Doom Blade", P1);
    state.move_object(opp_card, Zone::Graveyard, &reg);

    // Memory's Journey targeting P0's graveyard card.
    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P0), Target::Object(own_card)]);

    // P0's card should be in library, P1's card should still be in graveyard.
    assert_eq!(new_state.get_object(own_card).unwrap().zone, Zone::Library,
        "Own card should be shuffled into library");
    assert_eq!(new_state.get_object(opp_card).unwrap().zone, Zone::Graveyard,
        "Opponent's card should remain in their graveyard");
}

/// Has flashback for {G}.
#[test]
fn has_flashback() {
    let reg = registry();
    let id = reg.get_id_by_name("Memory's Journey").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.flashback_cost.is_some(), "Should have flashback cost");
    assert_eq!(data.flashback_cost.as_ref().unwrap().mana_value(), 1, "Flashback cost should be 1 (Green)");
}

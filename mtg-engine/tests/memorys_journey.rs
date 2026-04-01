//! Tests for Memory's Journey.
//!
//! Oracle: {1}{U} Instant
//! Target player shuffles up to three target cards from their graveyard into their library.
//! Flashback {G}

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Shuffles a card from your own graveyard into your library.
#[test]
fn shuffles_own_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(card, Zone::Graveyard);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Object(card)]);

    assert_eq!(new_state.get_object(card).unwrap().zone, Zone::Library,
        "Card should be shuffled into library");
}

/// Shuffles a card from opponent's graveyard into their library.
#[test]
fn shuffles_opponent_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(card, Zone::Graveyard);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Object(card)]);

    assert_eq!(new_state.get_object(card).unwrap().zone, Zone::Library,
        "Opponent's card should be shuffled into their library");
}

/// Can target up to 3 cards from the same graveyard.
#[test]
fn shuffles_up_to_three_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(card1, Zone::Graveyard);
    let card2 = spell_in_hand(&mut state, &reg, "Doom Blade", P0);
    state.move_object(card2, Zone::Graveyard);
    let card3 = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(card3, Zone::Graveyard);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![
        Target::Object(card1),
        Target::Object(card2),
        Target::Object(card3),
    ]);

    assert_eq!(new_state.get_object(card1).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(card2).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(card3).unwrap().zone, Zone::Library);
}

/// Legal actions should not mix cards from different graveyards.
#[test]
fn legal_actions_dont_mix_graveyards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(own_card, Zone::Graveyard);
    let opp_card = spell_in_hand(&mut state, &reg, "Doom Blade", P1);
    state.move_object(opp_card, Zone::Graveyard);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let actions = engine::legal_actions(&state, &reg);
    let cast_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if object_id == &journey)
    }).collect();

    // Should have single-target actions for each card (mode 1 and mode 2).
    assert!(cast_actions.len() >= 2, "Should have at least 2 cast actions");

    // Should NOT have any 2-target actions mixing both players' graveyards.
    let has_mixed = cast_actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            if targets.len() == 2 {
                let owners: Vec<_> = targets.iter().filter_map(|t| {
                    if let Target::Object(id) = t {
                        state.get_object(*id).map(|o| o.owner)
                    } else {
                        None
                    }
                }).collect();
                owners.len() == 2 && owners[0] != owners[1]
            } else {
                false
            }
        } else {
            false
        }
    });
    assert!(!has_mixed, "Should not mix cards from different graveyards");
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

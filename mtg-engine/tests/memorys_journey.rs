//! Tests for Memory's Journey.
//!
//! Oracle: {1}{U} Instant
//! Target player shuffles up to three target cards from their graveyard into their library.
//! Flashback {G}

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Shuffles a card from your own graveyard into your library.
#[test]
fn shuffles_own_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);

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

    let card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);

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

    let card1 = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let card2 = named_card_in_graveyard(&mut state, &reg, "Doom Blade", P0);
    let card3 = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

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

/// Resolving against one player's graveyard leaves the other's alone.
/// (Which cards are *offered* is checked in `multi_target_and_mill.rs`, on
/// `legal_actions`; this one is about what resolution actually moves.)
#[test]
fn resolving_only_moves_the_targeted_players_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own_card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let opp_card = named_card_in_graveyard(&mut state, &reg, "Doom Blade", P1);

    // Memory's Journey targeting P0's graveyard card.
    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P0), Target::Object(own_card)]);

    // P0's card should be in library, P1's card should still be in graveyard.
    assert_eq!(new_state.get_object(own_card).unwrap().zone, Zone::Library,
        "Own card should be shuffled into library");
    assert_eq!(new_state.get_object(opp_card).unwrap().zone, Zone::Graveyard,
        "Opponent's card should remain in their graveyard");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// "Target player shuffles up to three target cards from THEIR graveyard" —
/// the player is a target in its own right, and the cards are constrained by
/// it. Without the player slot the card offered every graveyard at once.
#[test]
fn memorys_journey_targets_a_player_and_then_their_cards() {
    use mtg_engine::cards::TargetRequirement;

    let registry = CardRegistry::with_all_cards();
    let behavior = registry
        .get(registry.get_id_by_name("Memory's Journey").unwrap())
        .unwrap();

    // Matched structurally rather than by searching the Debug string, so a
    // requirement that merely mentions a player somewhere cannot satisfy it.
    match behavior.target_requirement() {
        TargetRequirement::TwoTargets(first, second) => {
            assert!(matches!(*first, TargetRequirement::PlayerOnly),
                "the first target is the player whose graveyard it is, got {first:?}");
            match *second {
                TargetRequirement::UpToTargets(3, inner) => assert!(
                    matches!(*inner, TargetRequirement::GraveyardCardOwnedByTargetPlayer),
                    "the cards must be constrained to the targeted player's \
                     graveyard (CR 601.2c), got {inner:?}"),
                other => panic!("expected 'up to three' cards, got {other:?}"),
            }
        }
        other => panic!("expected TwoTargets(player, up-to-three cards), got {other:?}"),
    }
}

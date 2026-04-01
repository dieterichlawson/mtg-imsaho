//! Tests for Graveyard Shovel.
//!
//! Oracle: {2} Artifact
//! {2}, {T}: Target player exiles a card from their graveyard.
//! If it's a creature card, you gain 2 life.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Graveyard Shovel targets a player, not a card.
#[test]
fn targets_player_not_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);
    // Give P0 mana to activate.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    // Put a card in P1's graveyard.
    let gy_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(gy_card, Zone::Graveyard);

    let actions = engine::legal_actions(&state, &reg);
    let ability_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::ActivateAbility { object_id, .. } if object_id == &shovel)
    }).collect();

    // Should have ability actions targeting players.
    assert!(!ability_actions.is_empty(), "Should have ability actions");
    for action in &ability_actions {
        if let Action::ActivateAbility { targets, .. } = action {
            assert!(targets.iter().all(|t| matches!(t, Target::Player(_))),
                "Targets should be players, not cards");
        }
    }
}

/// When only one card in graveyard, it's auto-exiled.
#[test]
fn auto_exiles_single_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    // Put one creature in P1's graveyard.
    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(creature, Zone::Graveyard);

    let behavior = reg.get(state.get_object(shovel).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, shovel, 0, &[Target::Player(P1)], &reg);

    // Creature should be exiled.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile,
        "Single card should be auto-exiled");
    // Controller (P0) should gain 2 life (it was a creature).
    assert_eq!(state.get_player(P0).life, 22, "Should gain 2 life for creature");
}

/// When exiling a non-creature, no life gain.
#[test]
fn no_life_gain_for_non_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    // Put a non-creature in P1's graveyard.
    let noncreature = spell_in_hand(&mut state, &reg, "Doom Blade", P1);
    state.move_object(noncreature, Zone::Graveyard);

    let behavior = reg.get(state.get_object(shovel).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, shovel, 0, &[Target::Player(P1)], &reg);

    assert_eq!(state.get_object(noncreature).unwrap().zone, Zone::Exile,
        "Non-creature should be exiled");
    assert_eq!(state.get_player(P0).life, 20, "No life gain for non-creature");
}

/// With multiple cards in graveyard, sets up a resolution choice for the targeted player.
#[test]
fn multiple_cards_creates_resolution_choice() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    // Put two cards in P1's graveyard.
    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(creature, Zone::Graveyard);
    let instant = spell_in_hand(&mut state, &reg, "Doom Blade", P1);
    state.move_object(instant, Zone::Graveyard);

    let behavior = reg.get(state.get_object(shovel).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, shovel, 0, &[Target::Player(P1)], &reg);

    // Should have set up a resolution choice for P1 (the targeted player chooses).
    assert!(state.awaiting_action.is_some(), "Should set up a resolution choice");
    match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, .. }) => {
            assert_eq!(*player, P1, "Targeted player (P1) should choose which card to exile");
        }
        other => panic!("Expected ResolutionChoice, got {:?}", other),
    }
}

/// Resolution choice exiles the chosen card and gains life if creature.
#[test]
fn resolution_choice_exiles_and_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(creature, Zone::Graveyard);
    let instant = spell_in_hand(&mut state, &reg, "Doom Blade", P1);
    state.move_object(instant, Zone::Graveyard);

    // Activate ability targeting P1.
    let behavior = reg.get(state.get_object(shovel).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, shovel, 0, &[Target::Player(P1)], &reg);

    // Now resolve the choice — player chooses the creature.
    let new_state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(creature))),
        },
        &reg,
    );

    assert_eq!(new_state.get_object(creature).unwrap().zone, Zone::Exile,
        "Chosen creature should be exiled");
    assert_eq!(new_state.get_object(instant).unwrap().zone, Zone::Graveyard,
        "Unchosen card should remain in graveyard");
    assert_eq!(new_state.get_player(P0).life, 22,
        "Controller should gain 2 life (creature exiled)");
}

/// Can't target a player with no cards in graveyard.
#[test]
fn cannot_target_player_with_empty_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    // P1 has no cards in graveyard. P0 has one card.
    let gy_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(gy_card, Zone::Graveyard);

    let actions = engine::legal_actions(&state, &reg);
    let ability_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if object_id == &shovel && targets.iter().any(|t| matches!(t, Target::Player(p) if p == &P1)))
    }).collect();

    assert!(ability_actions.is_empty(),
        "Should not be able to target P1 (empty graveyard)");
}

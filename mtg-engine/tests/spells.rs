//! Tests for instants, sorceries, and targeting.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

/// Lightning Bolt deals 3 damage to a creature, killing a 3-toughness creature.
#[test]
fn lightning_bolt_kills_creature() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has a Lightning Bolt in hand and {R} in pool.
    let bolt_id = registry.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    // P1 has a 3/3 creature.
    let creature = ready_creature(&mut state, P1, 3, 3);

    // Cast Lightning Bolt targeting the creature.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(creature)] },
        &registry,
    );
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Stack);

    // Resolve it.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Creature should have 3 damage.
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3);
    // Bolt should be in graveyard.
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);

    // SBA should kill the creature.
    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Lightning Bolt deals 3 damage to a player.
#[test]
fn lightning_bolt_damages_player() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bolt_id = registry.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    assert_eq!(state.get_player(P1).life, 17);
}

/// Lightning Bolt can be cast at instant speed (during opponent's turn).
#[test]
fn instant_can_be_cast_during_opponent_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P1); // P1's turn
    state.priority_player = Some(P0); // but P0 has priority

    let bolt_id = registry.get_id_by_name("Lightning Bolt").unwrap();
    state.create_object(bolt_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &registry);
    let has_bolt = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(has_bolt, "Should be able to cast Lightning Bolt during opponent's turn");
}

/// Lightning Bolt can be cast during combat.
#[test]
fn instant_can_be_cast_during_combat() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    state.priority_player = Some(P0);

    let bolt_id = registry.get_id_by_name("Lightning Bolt").unwrap();
    state.create_object(bolt_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    // Need a creature to target.
    ready_creature(&mut state, P1, 2, 2);

    let legal = engine::legal_actions(&state, &registry);
    let has_bolt = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(has_bolt, "Should be able to cast Lightning Bolt during combat");
}

/// Sorceries can NOT be cast during opponent's turn.
#[test]
fn sorcery_cannot_be_cast_during_opponent_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P1); // P1's turn
    state.priority_player = Some(P0);

    let div_id = registry.get_id_by_name("Divination").unwrap();
    state.create_object(div_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 3);

    let legal = engine::legal_actions(&state, &registry);
    let has_div = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(!has_div, "Should not be able to cast sorcery during opponent's turn");
}

/// Sorceries can NOT be cast with a non-empty stack.
#[test]
fn sorcery_cannot_be_cast_with_nonempty_stack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let div_id = registry.get_id_by_name("Divination").unwrap();
    state.create_object(div_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 3);

    // Put something on the stack.
    let dummy = state.create_object(CardId(99), P0, Zone::Stack, None, None);
    state.stack.push(dummy);

    let legal = engine::legal_actions(&state, &registry);
    let has_div = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { .. }));
    assert!(!has_div, "Should not be able to cast sorcery with non-empty stack");
}

/// Divination draws two cards.
#[test]
fn divination_draws_two() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a library.
    let c1 = state.create_object(CardId(1), P0, Zone::Library, None, None);
    let c2 = state.create_object(CardId(1), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order = vec![c1, c2];

    let div_id = registry.get_id_by_name("Divination").unwrap();
    let div = state.create_object(div_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 3);

    // Cast and resolve.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: div, targets: vec![] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Should have drawn 2 cards.
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2);
    assert_eq!(state.get_object(div).unwrap().zone, Zone::Graveyard);
}

/// Swords to Plowshares exiles and gains life.
#[test]
fn swords_exiles_and_gains_life() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let swords_id = registry.get_id_by_name("Swords to Plowshares").unwrap();
    let swords = state.create_object(swords_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);

    // P1 has a 5/5 creature.
    let creature = ready_creature(&mut state, P1, 5, 5);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: swords, targets: vec![Target::Object(creature)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Creature should be exiled.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile);
    // P1 gains 5 life.
    assert_eq!(state.get_player(P1).life, 25);
    // Swords in graveyard.
    assert_eq!(state.get_object(swords).unwrap().zone, Zone::Graveyard);
}

/// Doom Blade destroys a creature.
#[test]
fn doom_blade_destroys() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doom_id = registry.get_id_by_name("Doom Blade").unwrap();
    let doom = state.create_object(doom_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let creature = ready_creature(&mut state, P1, 10, 10);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: doom, targets: vec![Target::Object(creature)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Targeted spells need valid targets — can't cast Lightning Bolt with no creatures or players.
#[test]
fn targeted_spell_needs_valid_target() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Doom Blade targets creatures — if none exist, can't cast.
    let doom_id = registry.get_id_by_name("Doom Blade").unwrap();
    state.create_object(doom_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let legal = engine::legal_actions(&state, &registry);
    let has_doom = legal.actions.iter().any(|a| match a {
        Action::CastSpell { object_id, .. } => {
            state.get_object(*object_id).map(|o| o.card_id) == Some(doom_id)
        }
        _ => false,
    });
    assert!(!has_doom, "Should not be able to cast Doom Blade with no creatures on battlefield");
}

/// Lava Axe deals 5 to a player.
#[test]
fn lava_axe_damages_player() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let axe_id = registry.get_id_by_name("Lava Axe").unwrap();
    let axe = state.create_object(axe_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 4);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: axe, targets: vec![Target::Player(P1)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    assert_eq!(state.get_player(P1).life, 15);
    assert_eq!(state.get_object(axe).unwrap().zone, Zone::Graveyard);
}

//! Tests for zone rules, game state immutability, and object tracking.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

/// Rule 400.3: Objects always go to their OWNER's graveyard/hand/library,
/// even if controlled by another player.
#[test]
fn objects_go_to_owners_graveyard() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 owns a creature, but P1 controls it.
    let creature = state.create_object(CardId(99), P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().controller = P1;

    // Battlefield filters by controller.
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P0).len(), 0);
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P1).len(), 1);

    // When it dies, it goes to the OWNER's graveyard.
    state.move_object(creature, Zone::Graveyard);
    assert_eq!(state.objects_in_zone(Zone::Graveyard, P0).len(), 1,
        "Card should go to owner's graveyard (rule 400.3)");
    assert_eq!(state.objects_in_zone(Zone::Graveyard, P1).len(), 0);
}

/// Hand zone filters by owner (rule 400.1).
#[test]
fn hand_filters_by_owner() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.create_object(CardId(1), P0, Zone::Hand, None, None);
    state.create_object(CardId(2), P0, Zone::Hand, None, None);
    state.create_object(CardId(1), P1, Zone::Hand, None, None);

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2);
    assert_eq!(state.objects_in_zone(Zone::Hand, P1).len(), 1);
}

/// Verify that submit_action returns a new state without modifying the original.
#[test]
fn submit_action_preserves_original_state() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

    let original_hand_size = state.objects_in_zone(Zone::Hand, P0).len();

    let new_state = engine::submit_action(
        &state, &Action::PlayLand { object_id: land }, &registry,
    );

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), original_hand_size,
        "Original state should not be modified");
    assert_eq!(state.get_object(land).unwrap().zone, Zone::Hand);
    assert_eq!(new_state.get_object(land).unwrap().zone, Zone::Battlefield);
}

/// Zone change counter increments on each zone change.
#[test]
fn zone_change_counter_increments() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 0);

    state.move_object(creature, Zone::Battlefield);
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 1);

    state.move_object(creature, Zone::Graveyard);
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 2);

    state.move_object(creature, Zone::Exile);
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 3);
}

/// Leaving the battlefield resets tapped, damage, and summoning sickness.
#[test]
fn leaving_battlefield_resets_state() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    state.get_object_mut(creature).unwrap().tapped = true;
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    state.move_object(creature, Zone::Graveyard);

    let obj = state.get_object(creature).unwrap();
    assert!(!obj.tapped);
    assert_eq!(obj.damage_marked, 0);
    assert!(!obj.summoning_sick);
}

/// Creature spell goes on the stack, not directly to battlefield.
#[test]
fn creature_spell_goes_on_stack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = castable_spell(&mut state, &registry, "Kalonian Tusker", P0);

    let new_state = engine::submit_action(
        &state, &Action::CastSpell { object_id: creature, targets: vec![], sacrifice: None, exile_count: None, alternative_cost: None }, &registry,
    );

    assert_eq!(new_state.get_object(creature).unwrap().zone, Zone::Stack);
    assert_eq!(new_state.stack.len(), 1);
}

/// Creature resolves to battlefield with summoning sickness.
#[test]
fn creature_resolves_with_summoning_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = castable_spell(&mut state, &registry, "Kalonian Tusker", P0);

    state = cast_and_resolve(&state, &registry, creature, vec![]);

    let obj = state.get_object(creature).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(obj.summoning_sick);
}

// ============================================================================
// Full integration: tap lands, cast, resolve
// ============================================================================

/// End-to-end: tap two Forests, cast Kalonian Tusker, resolve it.
#[test]
fn full_cast_and_resolve_sequence() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = registry.get_id_by_name("Forest").unwrap();

    let forest1 = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(forest1).unwrap().summoning_sick = false;
    let forest2 = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(forest2).unwrap().summoning_sick = false;
    let tusker = spell_in_hand(&mut state, &registry, "Kalonian Tusker", P0);

    // Tap Forest 1.
    state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility { object_id: forest1, ability_index: 0 },
        &registry,
    );
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Green), 1);

    // Tap Forest 2.
    state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility { object_id: forest2, ability_index: 0 },
        &registry,
    );
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Green), 2);

    // Cast Kalonian Tusker.
    state = engine::submit_action(
        &state, &Action::CastSpell { object_id: tusker, targets: vec![], sacrifice: None, exile_count: None, alternative_cost: None }, &registry,
    );
    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Stack);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);

    // Resolve.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    let obj = state.get_object(tusker).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(obj.summoning_sick);
    assert_eq!(obj.power, Some(3));
    assert_eq!(obj.toughness, Some(3));
    assert!(state.stack.is_empty());
}

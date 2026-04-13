mod common;

use common::{game_at_step, P0, P1};
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::types::*;

/// Bug: Grimoire of the Dead emits EnteredBattlefield events with the wrong
/// controller for opponent's creatures. The move_object call at
/// grimoire_of_the_dead.rs:160 fires before the controller is updated at
/// line 162, so the ETB event records the creature's original owner (P1)
/// instead of the Grimoire's controller (P0).
///
/// Oracle text says "Put all creature cards from all graveyards onto the
/// battlefield under your control." The ETB event should record P0 as
/// controller for every returned creature, regardless of which graveyard
/// it came from.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn bug_grimoire_etb_event_wrong_controller_for_opponent_creature() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Grimoire of the Dead on P0's battlefield with 3 study counters.
    let grimoire_card_id = registry.get_id_by_name("Grimoire of the Dead")
        .expect("Grimoire of the Dead must be registered");
    let grimoire = state.create_object(grimoire_card_id, P0, Zone::Battlefield, None, None);
    if let Some(obj) = state.get_object_mut(grimoire) {
        obj.name = "Grimoire of the Dead".into();
        obj.is_legendary = true;
    }
    state.add_counters(grimoire, CounterType::Study, 3);

    // Place an opponent creature in P1's graveyard.
    let creature_in_gy = state.create_object(
        mtg_engine::ids::CardId(8000), P1, Zone::Graveyard, Some(3), Some(3),
    );
    if let Some(obj) = state.get_object_mut(creature_in_gy) {
        obj.name = "Opponent Creature".into();
        obj.card_types.push(CardType::Creature);
    }

    // Clear any setup events so we only see ability-generated ones.
    state.events.clear();

    // Activate ability 1: {T}, remove 3 study counters, sacrifice Grimoire.
    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: grimoire,
            ability_index: 1,
            targets: vec![],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
        },
        &registry,
    );

    // Find the EnteredBattlefield event for the opponent's creature.
    let etb_events: Vec<&GameEvent> = new_state.events.iter()
        .filter(|e| matches!(e, GameEvent::EnteredBattlefield { object, .. } if *object == creature_in_gy))
        .collect();

    assert!(!etb_events.is_empty(), "Expected an EnteredBattlefield event for the returned creature");

    // The Oracle text says the creature enters under YOUR (P0's) control.
    // The ETB event should record P0 as the controller.
    // BUG: move_object fires before controller is updated, so the event
    // records P1 (the original owner) instead of P0 (the Grimoire controller).
    for event in &etb_events {
        if let GameEvent::EnteredBattlefield { controller, .. } = event {
            assert_eq!(
                *controller, P0,
                "EnteredBattlefield event should record P0 as controller for creature \
                 returned by Grimoire of the Dead, but got P1 (the original owner). \
                 move_object emits the event before the controller is updated."
            );
        }
    }
}

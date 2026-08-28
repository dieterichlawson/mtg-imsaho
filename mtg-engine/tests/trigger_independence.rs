//! A triggered ability, once on the stack, is independent of its source
//! (CR 113.7a) — and the conditions in the trigger's own wording are read when
//! the event happens (CR 603.2), not when it resolves.

mod common;

use common::*;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::{GameState, StackEntry};
use mtg_engine::triggers::{self, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;
fn damage_watch_triggers(state: &GameState, watcher: ObjectId) -> usize {
    state.stack.iter()
        .filter(|e| matches!(e, StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: watcher_id, .. },
                event: TriggerEvent::AnyDamageToPlayer { .. } }) if *watcher_id == watcher))
        .count()
}

/// Curiosity reads "whenever ENCHANTED CREATURE deals damage to AN OPPONENT".
/// Both halves are part of the triggering event.
#[test]
fn curiosity_only_triggers_for_its_own_creature_damaging_an_opponent() {
    let reg = registry();

    // The enchanted creature damaging an opponent: triggers.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let curiosity = named_permanent(&mut state, &reg, "Curiosity", P0);
    state.get_object_mut(curiosity).unwrap().attached_to = Some(bear);
    state.events.push(mtg_engine::events::GameEvent::NonCombatDamageDealt {
        source: bear,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(damage_watch_triggers(&state, curiosity), 1,
        "the enchanted creature damaged an opponent, so Curiosity triggers");

    // A DIFFERENT creature damaging an opponent: must not trigger.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let other = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let curiosity = named_permanent(&mut state, &reg, "Curiosity", P0);
    state.get_object_mut(curiosity).unwrap().attached_to = Some(bear);
    state.events.push(mtg_engine::events::GameEvent::NonCombatDamageDealt {
        source: other,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(damage_watch_triggers(&state, curiosity), 0,
        "a creature other than the enchanted one damaged the opponent — \
         Curiosity must not go on the stack at all (CR 603.2)");

    // The enchanted creature damaging its OWN controller: must not trigger.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let curiosity = named_permanent(&mut state, &reg, "Curiosity", P0);
    state.get_object_mut(curiosity).unwrap().attached_to = Some(bear);
    state.events.push(mtg_engine::events::GameEvent::NonCombatDamageDealt {
        source: bear,
        target: mtg_engine::events::DamageTarget::Player(P0),
        amount: 2,
    });
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(damage_watch_triggers(&state, curiosity), 0,
        "damage to its own controller is not damage to 'an opponent'");
}

/// CR 113.7a: destroying the source in response does not counter the ability.
#[test]
fn reapers_end_step_trigger_resolves_after_the_reaper_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    state.creature_died_this_turn = true; // morbid satisfied

    let reaper = named_permanent(&mut state, &reg, "Reaper from the Abyss", P0);
    let victim = named_permanent(&mut state, &reg, "Walking Corpse", P1);

    // The trigger resolves with the Reaper already destroyed.
    let behavior = reg.get(state.get_object(reaper).unwrap().card_id).unwrap();
    mtg_engine::destruction::try_destroy(&mut state, reaper, &reg);
    assert_ne!(state.get_object(reaper).unwrap().zone, Zone::Battlefield, "test precondition");

    behavior.on_end_step(&mut state, reaper, &[mtg_engine::actions::Target::Object(victim)], &reg);

    assert_ne!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "the ability is on the stack independently of the Reaper, so killing \
         the Reaper in response must not save the target (CR 113.7a)");
}

/// A non-creature watcher destroyed alongside the creature it watches still
/// sees the death. `destroy` only emits CreatureDied for things with power, so
/// an enchantment was invisible to the simultaneous-death list.
#[test]
fn a_non_creature_watcher_destroyed_simultaneously_still_triggers() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    let creature = named_permanent(&mut state, &reg, "Walking Corpse", P0);

    // Both destroyed in the same batch.
    mtg_engine::destruction::try_destroy(&mut state, creature, &reg);
    mtg_engine::destruction::try_destroy(&mut state, grime, &reg);
    triggers::collect_triggers(&mut state, &reg);

    let triggered = state.stack.iter().any(|e| matches!(e, StackEntry::Trigger(
        PendingTrigger {
            source: TriggerSource { id: watcher_id, .. },
            event: TriggerEvent::CreatureDied { .. } }) if *watcher_id == grime));
    assert!(triggered,
        "Gutter Grime is an enchantment, so no CreatureDied event is emitted \
         for it — but it was on the battlefield when the creature died and its \
         death-watch ability must still trigger");
}

/// CR 121.1: a counter aimed at a permanent that has left the battlefield
/// lands nowhere.
#[test]
fn counters_are_not_added_to_a_permanent_that_has_left_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    state.move_object(grime, Zone::Graveyard, &reg);

    state.add_counters(grime, CounterType::Slime, 1);

    assert_eq!(counters_of(&state, grime, CounterType::Slime), 0,
        "the Gutter Grime in the graveyard is a different object; a slime \
         counter there would make its Ooze token 1/1 instead of the 0/0 the \
         ruling requires, and would ride along if it were reanimated");
}

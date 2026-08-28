//! Triggers from a permanent entering or leaving a zone.

use super::Collector;
use super::super::*;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::Zone;

pub(super) fn entered_battlefield(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let GameEvent::EnteredBattlefield { object, .. } = event else { return };
    // Per MTG rules, ETB triggers fire even if the source has since
    // left the battlefield. The trigger was created by the entering event.
    let (card_id, controller) = match state.get_object(*object) {
        Some(o) => (o.card_id, o.controller),
        _ => return,
    };
    // Only collect if the card actually has an ETB handler.
    // Cards without one (basic lands, vanilla creatures) shouldn't put a
    // trigger on the stack — per MTG rules, only declared triggered abilities
    // create stack entries.
    if let Some(behavior) = registry.get(card_id) {
        // CR 603.4: an intervening-if condition is checked here, when
        // the ability would trigger — a false condition means no stack
        // entry and no priority window at all.
        let etb_kind = crate::cards::TriggerKind::EntersBattlefield;
        if behavior.has_etb_handler()
            && behavior.should_trigger(state, *object, &etb_kind, registry)
        {
            let desc = trigger_description(registry, card_id, &etb_kind, false);
            c.emit(*object, card_id, controller, desc, TriggerEvent::SelfEntered);
        }
    }

    // ETB-watch: notify other permanents (and graveyard cards like Dearly Departed)
    // that a creature entered. Only collect if the watcher's zone matches
    // the trigger's allowed zones (via CardBehavior::trigger_zones).
    if state.is_creature(*object, registry) {
        let watchers: Vec<(ObjectId, CardId, PlayerId, Zone)> = state.objects_in_id_order().into_iter()
            .filter(|o| (o.zone == Zone::Battlefield || o.zone == Zone::Graveyard) && o.id != *object)
            .map(|o| (o.id, o.card_id, o.controller, o.zone))
            .collect();
        let trigger_kind = crate::cards::TriggerKind::AnyCreatureEnters;
        for (watcher_id, watcher_card_id, watcher_controller, watcher_zone) in watchers {
            if let Some(behavior) = registry.get(watcher_card_id) {
                let has_trigger = behavior.card_data().triggered_abilities.iter()
                    .any(|t| t.kind == trigger_kind);
                // CR 603.2: an event condition on the entering
                // creature is read HERE, as it enters — not at
                // resolution, by which time its power may differ.
                let condition_holds = behavior.should_trigger_on_creature_enters(
                    state, watcher_id, *object, controller, registry);
                if has_trigger && condition_holds
                    && behavior.trigger_zones(&trigger_kind).contains(&watcher_zone) {
                    let desc = trigger_description(registry, watcher_card_id, &trigger_kind, false);
                    c.emit(watcher_id, watcher_card_id, watcher_controller, desc,
                        TriggerEvent::CreatureEntered {
                            entered: *object,
                            entered_controller: controller,
                        });
                }
            }
        }
    }
}

pub(super) fn creature_died(
    state: &mut GameState,
    events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let GameEvent::CreatureDied { object, card_id, controller, damaged_by, last_known_toughness, is_token } = event else { return };
    let dead_id = *object;
    let dead_card_id = *card_id;
    let dead_controller = *controller;
    let dead = DeadCreature {
        id: dead_id,
        controller: dead_controller,
        damaged_by: damaged_by.clone(),
        toughness: *last_known_toughness,
        is_token: *is_token,
    };

    // 1. Self-dies trigger. Only fire if the card actually has a
    // SelfDies TriggeredAbilityDef — vanilla creatures and creatures
    // with only watcher/ETB/activated abilities must not pollute the
    // stack with empty triggers.
    if card_has_trigger(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies) {
        let desc = trigger_description(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies, false);
        c.emit(dead_id, dead_card_id, dead_controller, desc, TriggerEvent::SelfDies);
    }

    // 2. Death-watch: collect triggers from permanents on the
    // battlefield, plus permanents that left in the same event
    // batch — they were still on the battlefield when the
    // simultaneous deaths occurred, so their death-watch abilities
    // trigger (CR 603.10a).
    //
    // Keyed on LeftBattlefield rather than CreatureDied, because
    // `destroy` only emits CreatureDied for things with power. A
    // NON-creature watcher destroyed alongside the creature it
    // watches — Gutter Grime is an enchantment — was invisible to
    // this list and lost its trigger entirely.
    let watchers: Vec<(ObjectId, CardId, PlayerId, bool)> = state.objects_in_id_order().into_iter()
        .filter(|o| o.id != dead_id && super::was_on_the_battlefield(state, events, o.id))
        .map(|o| (o.id, o.card_id, o.controller, o.is_transformed))
        .collect();
    for (watcher_id, watcher_card_id, watcher_controller, watcher_transformed) in watchers {
        // Only create death-watch triggers for permanents whose ACTIVE
        // face has an AnyCreatureDies triggered ability (CR 712.8d —
        // a transformed DFC only has its back face's abilities).
        let has_death_trigger = state.triggered_abilities_of(watcher_id, registry).iter()
            .any(|t| t.kind == crate::cards::TriggerKind::AnyCreatureDies);
        if has_death_trigger {
            let desc = face_trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCreatureDies, watcher_transformed);
            c.emit(watcher_id, watcher_card_id, watcher_controller, desc,
                TriggerEvent::CreatureDied { dead: dead.clone() });
        }
    }
}

pub(super) fn left_battlefield(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let GameEvent::LeftBattlefield { object, last_controller, .. } = event else { return };
    let (card_id,) = match state.get_object(*object) {
        Some(o) => (o.card_id,),
        None => return,
    };
    // Only fire LTB triggers for cards that actually have a
    // LeavesBattlefield TriggeredAbilityDef. Vanilla creatures,
    // auras without LTB clauses (Bonds of Faith, Dead Weight),
    // and other non-LTB permanents must not pollute the stack.
    if card_has_trigger(registry, card_id, &crate::cards::TriggerKind::LeavesBattlefield) {
        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::LeavesBattlefield, false);
        c.emit(*object, card_id, *last_controller, desc, TriggerEvent::LeftBattlefield);
    }
}

pub(super) fn creature_card_milled(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let GameEvent::CreatureCardMilled { object, milled_player } = event else { return };
    let milled_obj = *object;
    let milled_player = *milled_player;
    // Find watchers on the battlefield with CreatureCardMilled triggers.
    let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects_in_id_order().into_iter()
        .filter(|o| o.zone == Zone::Battlefield)
        .map(|o| (o.id, o.card_id, o.controller))
        .collect();
    for (watcher_id, watcher_card_id, watcher_controller) in watchers {
        // Only watchers who are opponents of the milled player.
        if watcher_controller == milled_player { continue; }
        let has_trigger = registry.get(watcher_card_id)
            .is_some_and(|b| b.card_data().triggered_abilities.iter()
                .any(|t| t.kind == crate::cards::TriggerKind::CreatureCardMilled));
        if has_trigger {
            let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::CreatureCardMilled, false);
            c.emit(watcher_id, watcher_card_id, watcher_controller, desc,
                TriggerEvent::CreatureCardMilled { milled_object: milled_obj, milled_player });
        }
    }
}

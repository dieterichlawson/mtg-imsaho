use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::Zone;

/// A triggered ability that has been collected but not yet resolved.
/// These are placed on pending_triggers in APNAP order, then resolved
/// LIFO (non-active player's triggers resolve first).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PendingTrigger {
    /// A creature's own "when this dies" trigger.
    SelfDies {
        /// The creature that died (now in graveyard).
        dead_id: ObjectId,
        /// The card ID for looking up behavior.
        dead_card_id: CardId,
        /// Who controlled the creature when it died.
        controller: PlayerId,
    },
    /// A "whenever a creature dies" death-watch trigger on another permanent.
    DeathWatch {
        /// The permanent with the triggered ability.
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        /// Who controls the watcher.
        controller: PlayerId,
        /// The creature that died.
        dead_id: ObjectId,
        dead_controller: PlayerId,
    },
    /// A creature entering the battlefield trigger.
    EnteredBattlefield {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
    },
    /// A permanent leaving the battlefield trigger.
    LeftBattlefield {
        object_id: ObjectId,
        card_id: CardId,
    },
}

impl PendingTrigger {
    /// The player who controls this trigger.
    pub fn controller(&self) -> PlayerId {
        match self {
            PendingTrigger::SelfDies { controller, .. } => *controller,
            PendingTrigger::DeathWatch { controller, .. } => *controller,
            PendingTrigger::EnteredBattlefield { controller, .. } => *controller,
            PendingTrigger::LeftBattlefield { .. } => PlayerId(255), // ETB/LTB triggers are handled by the card
        }
    }
}

/// Collect triggered abilities from events and add them to state.pending_triggers
/// in APNAP order (active player first on bottom, non-active player on top).
///
/// Does NOT resolve them — the game loop resolves them one at a time,
/// giving players priority between each.
pub fn collect_triggers(state: &mut GameState, registry: &CardRegistry) {
    let events = state.events.clone();
    let start = state.trigger_event_index;
    let active_player = state.active_player;

    let mut ap_triggers: Vec<PendingTrigger> = Vec::new();
    let mut nap_triggers: Vec<PendingTrigger> = Vec::new();

    for (i, event) in events.iter().enumerate().skip(start) {
        match event {
            GameEvent::EnteredBattlefield { object, .. } => {
                let (card_id, controller) = match state.get_object(*object) {
                    Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
                    _ => continue,
                };
                // Only collect if the card has an on_enter_battlefield handler.
                if registry.get(card_id).is_some() {
                    let trigger = PendingTrigger::EnteredBattlefield {
                        object_id: *object,
                        card_id,
                        controller,
                    };
                    if controller == active_player {
                        ap_triggers.push(trigger);
                    } else {
                        nap_triggers.push(trigger);
                    }
                }
            }
            GameEvent::CreatureDied { object, card_id, controller } => {
                let dead_id = *object;
                let dead_card_id = *card_id;
                let dead_controller = *controller;

                // 1. Self-dies trigger.
                if registry.get(dead_card_id).is_some() {
                    let trigger = PendingTrigger::SelfDies {
                        dead_id,
                        dead_card_id,
                        controller: dead_controller,
                    };
                    if dead_controller == active_player {
                        ap_triggers.push(trigger);
                    } else {
                        nap_triggers.push(trigger);
                    }
                }

                // 2. Death-watch: collect triggers from all permanents on battlefield.
                let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)
                    .map(|o| (o.id, o.card_id, o.controller))
                    .collect();
                for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                    if registry.get(watcher_card_id).is_some() {
                        let trigger = PendingTrigger::DeathWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            dead_id,
                            dead_controller,
                        };
                        if watcher_controller == active_player {
                            ap_triggers.push(trigger);
                        } else {
                            nap_triggers.push(trigger);
                        }
                    }
                }
            }
            GameEvent::LeftBattlefield { object, .. } => {
                let (card_id,) = match state.get_object(*object) {
                    Some(o) => (o.card_id,),
                    None => continue,
                };
                if registry.get(card_id).is_some() {
                    let trigger = PendingTrigger::LeftBattlefield {
                        object_id: *object,
                        card_id,
                    };
                    // LTB triggers go on AP side (they're usually self-referential).
                    ap_triggers.push(trigger);
                }
            }
            _ => {}
        }
    }

    // APNAP: Active player's triggers go on stack first (bottom),
    // non-active player's go on top. Since pending_triggers is a Vec
    // and we resolve from the back (LIFO), AP goes first in the Vec.
    state.pending_triggers.extend(ap_triggers);
    state.pending_triggers.extend(nap_triggers);

    // Mark all events as processed.
    state.trigger_event_index = events.len();
}

/// Resolve the next pending trigger (the last one in the list = top of "stack").
/// Returns true if a trigger was resolved, false if the queue is empty.
pub fn resolve_next_trigger(state: &mut GameState, registry: &CardRegistry) -> bool {
    let trigger = match state.pending_triggers.pop() {
        Some(t) => t,
        None => return false,
    };

    match trigger {
        PendingTrigger::EnteredBattlefield { object_id, card_id, .. } => {
            // Verify the object is still on the battlefield.
            if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_enter_battlefield(state, object_id, registry);
                }
            }
        }
        PendingTrigger::SelfDies { dead_id, dead_card_id, .. } => {
            if let Some(behavior) = registry.get(dead_card_id) {
                behavior.on_dies(state, dead_id, registry);
            }
        }
        PendingTrigger::DeathWatch { watcher_id, watcher_card_id, dead_id, dead_controller, .. } => {
            // Verify the watcher is still on the battlefield.
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_creature_dies(state, watcher_id, dead_id, dead_controller, registry);
                }
            }
        }
        PendingTrigger::LeftBattlefield { object_id, card_id } => {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_leave_battlefield(state, object_id, registry);
            }
        }
    }

    true
}

/// Legacy: process all triggers synchronously (used by tests that don't go
/// through the game loop). Collects and immediately resolves all triggers.
pub fn process_triggers(state: &mut GameState, registry: &CardRegistry) {
    collect_triggers(state, registry);

    // Resolve all pending triggers in LIFO order.
    while resolve_next_trigger(state, registry) {
        // If a trigger set an awaiting_action, pause and let the caller handle it.
        if state.awaiting_action.is_some() {
            return;
        }

        // Collect any new triggers that the resolution may have caused.
        collect_triggers(state, registry);
    }

    state.trigger_event_index = 0;
}

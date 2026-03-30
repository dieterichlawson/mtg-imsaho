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
        dead_id: ObjectId,
        dead_card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// A "whenever a creature dies" death-watch trigger on another permanent.
    DeathWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        dead_id: ObjectId,
        dead_controller: PlayerId,
        description: String,
    },
    /// A creature entering the battlefield trigger.
    EnteredBattlefield {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// A "whenever a creature enters the battlefield" ETB-watch trigger on another permanent.
    EnterWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        entered_id: ObjectId,
        entered_controller: PlayerId,
        description: String,
    },
    /// A creature dealt combat damage to a player.
    CombatDamageToPlayer {
        creature_id: ObjectId,
        creature_card_id: CardId,
        controller: PlayerId,
        damaged_player: PlayerId,
        amount: u32,
        description: String,
    },
    /// A permanent leaving the battlefield trigger.
    LeftBattlefield {
        object_id: ObjectId,
        card_id: CardId,
        description: String,
    },
}

impl PendingTrigger {
    /// The player who controls this trigger.
    pub fn controller(&self) -> PlayerId {
        match self {
            PendingTrigger::SelfDies { controller, .. } => *controller,
            PendingTrigger::DeathWatch { controller, .. } => *controller,
            PendingTrigger::EnteredBattlefield { controller, .. } => *controller,
            PendingTrigger::EnterWatch { controller, .. } => *controller,
            PendingTrigger::CombatDamageToPlayer { controller, .. } => *controller,
            PendingTrigger::LeftBattlefield { .. } => PlayerId(255),
        }
    }

    /// Display name for the stack view, including what the trigger does.
    pub fn display_name(&self, registry: &crate::cards::CardRegistry) -> String {
        let card_name = |card_id: CardId| {
            registry.card_data(card_id)
                .map(|d| d.name)
                .unwrap_or_else(|| "Unknown".into())
        };
        match self {
            PendingTrigger::SelfDies { dead_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s dies trigger", card_name(*dead_card_id))
                } else {
                    format!("{}'s dies trigger ({})", card_name(*dead_card_id), description)
                }
            }
            PendingTrigger::DeathWatch { watcher_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s triggered ability", card_name(*watcher_card_id))
                } else {
                    format!("{}'s triggered ability ({})", card_name(*watcher_card_id), description)
                }
            }
            PendingTrigger::EnteredBattlefield { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s ETB trigger", card_name(*card_id))
                } else {
                    format!("{}'s ETB trigger ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::EnterWatch { watcher_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s triggered ability", card_name(*watcher_card_id))
                } else {
                    format!("{}'s triggered ability ({})", card_name(*watcher_card_id), description)
                }
            }
            PendingTrigger::CombatDamageToPlayer { creature_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s combat damage trigger", card_name(*creature_card_id))
                } else {
                    format!("{}'s combat damage trigger ({})", card_name(*creature_card_id), description)
                }
            }
            PendingTrigger::LeftBattlefield { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s LTB trigger", card_name(*card_id))
                } else {
                    format!("{}'s LTB trigger ({})", card_name(*card_id), description)
                }
            }
        }
    }
}

/// Look up the description for a trigger from the card's TriggeredAbilityDef.
fn trigger_description(registry: &CardRegistry, card_id: CardId, kind: &crate::cards::TriggerKind) -> String {
    registry.card_data(card_id)
        .and_then(|d| d.triggered_abilities.iter()
            .find(|t| &t.kind == kind)
            .map(|t| t.description.clone()))
        .unwrap_or_default()
}

/// Collect triggered abilities from events and add them to the stack
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
                // Self ETB trigger.
                if registry.get(card_id).is_some() {
                    let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::EntersBattlefield);
                    let trigger = PendingTrigger::EnteredBattlefield {
                        object_id: *object,
                        card_id,
                        controller,
                        description: desc,
                    };
                    if controller == active_player {
                        ap_triggers.push(trigger);
                    } else {
                        nap_triggers.push(trigger);
                    }
                }

                // ETB-watch: notify other permanents that a creature entered.
                if state.get_object(*object).map(|o| o.power.is_some()).unwrap_or(false) {
                    let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield && o.id != *object)
                        .map(|o| (o.id, o.card_id, o.controller))
                        .collect();
                    for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                        if registry.get(watcher_card_id).is_some() {
                            let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCreatureEnters);
                            if !desc.is_empty() {
                                let trigger = PendingTrigger::EnterWatch {
                                    watcher_id,
                                    watcher_card_id,
                                    controller: watcher_controller,
                                    entered_id: *object,
                                    entered_controller: controller,
                                    description: desc,
                                };
                                if watcher_controller == active_player {
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
                                }
                            }
                        }
                    }
                }
            }
            GameEvent::CreatureDied { object, card_id, controller } => {
                let dead_id = *object;
                let dead_card_id = *card_id;
                let dead_controller = *controller;

                // 1. Self-dies trigger.
                if registry.get(dead_card_id).is_some() {
                    let desc = trigger_description(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies);
                    let trigger = PendingTrigger::SelfDies {
                        dead_id,
                        dead_card_id,
                        controller: dead_controller,
                        description: desc,
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
                        let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCreatureDies);
                        let trigger = PendingTrigger::DeathWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            dead_id,
                            dead_controller,
                            description: desc,
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
                    let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::LeavesBattlefield);
                    let trigger = PendingTrigger::LeftBattlefield {
                        object_id: *object,
                        card_id,
                        description: desc,
                    };
                    // LTB triggers go on AP side (they're usually self-referential).
                    ap_triggers.push(trigger);
                }
            }
            GameEvent::CombatDamageDealt { source, target, amount } => {
                // Only trigger for creature-to-player combat damage.
                if let crate::events::DamageTarget::Player(damaged_player) = target {
                    let source_id = *source;
                    if let Some(obj) = state.get_object(source_id) {
                        if obj.zone == Zone::Battlefield && obj.power.is_some() {
                            let card_id = obj.card_id;
                            let controller = obj.controller;
                            if registry.get(card_id).is_some() {
                                let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::CombatDamageToPlayer);
                                if !desc.is_empty() {
                                    let trigger = PendingTrigger::CombatDamageToPlayer {
                                        creature_id: source_id,
                                        creature_card_id: card_id,
                                        controller,
                                        damaged_player: *damaged_player,
                                        amount: *amount,
                                        description: desc,
                                    };
                                    if controller == active_player {
                                        ap_triggers.push(trigger);
                                    } else {
                                        nap_triggers.push(trigger);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // APNAP: Active player's triggers go on stack first (bottom),
    // non-active player's go on top. LIFO = NAP resolves first.
    use crate::state::StackEntry;
    for t in ap_triggers {
        state.stack.push(StackEntry::Trigger(t));
    }
    for t in nap_triggers {
        state.stack.push(StackEntry::Trigger(t));
    }

    // Mark all events as processed.
    state.trigger_event_index = events.len();
}

/// Resolve the top trigger from the stack.
/// Returns true if a trigger was resolved, false if the top of stack is not a trigger.
pub fn resolve_next_trigger(state: &mut GameState, registry: &CardRegistry) -> bool {
    // Check if the top of stack is a trigger.
    let is_trigger = state.stack.last()
        .map(|e| matches!(e, crate::state::StackEntry::Trigger(_)))
        .unwrap_or(false);
    if !is_trigger {
        return false;
    }
    let entry = state.stack.pop().expect("stack must have trigger entry");
    let trigger = match entry {
        crate::state::StackEntry::Trigger(t) => t,
        _ => unreachable!(),
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
        PendingTrigger::EnterWatch { watcher_id, watcher_card_id, entered_id, entered_controller, .. } => {
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_creature_enters(state, watcher_id, entered_id, entered_controller, registry);
                }
            }
        }
        PendingTrigger::CombatDamageToPlayer { creature_id, creature_card_id, damaged_player, amount, .. } => {
            // Creature may have died since the trigger was put on the stack, but
            // the trigger still resolves (it's independent on the stack).
            if let Some(behavior) = registry.get(creature_card_id) {
                behavior.on_combat_damage_to_player(state, creature_id, damaged_player, amount, registry);
            }
        }
        PendingTrigger::LeftBattlefield { object_id, card_id, .. } => {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_leave_battlefield(state, object_id, registry);
            }
        }
    }

    true
}

/// Process all triggers synchronously: collect from events, push to stack,
/// and resolve all triggers in LIFO order. Used by tests and code that
/// doesn't go through the full game loop.
pub fn process_triggers(state: &mut GameState, registry: &CardRegistry) {
    collect_triggers(state, registry);

    // Resolve all triggers from the stack in LIFO order.
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

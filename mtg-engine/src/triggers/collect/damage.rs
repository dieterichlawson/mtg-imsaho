//! Triggers from damage being dealt.

use super::Collector;
use super::super::*;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::Zone;

pub(super) fn combat_damage(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let active_player = c.active_player;
    let GameEvent::CombatDamageDealt { source, target, amount } = event else { return };
        // Creature-to-creature combat damage triggers.
        if let crate::events::DamageTarget::Object(damaged_id) = target {
            let source_id = *source;
            if let Some(obj) = state.get_object(source_id) {
                if obj.zone == Zone::Battlefield && state.is_creature(source_id, registry) {
                    let card_id = obj.card_id;
                    let controller = obj.controller;
                    if registry.get(card_id).is_some() {
                        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::DealsCombatDamageToCreature, false);
                        if !desc.is_empty() {
                            let trigger = PendingTrigger::CombatDamageToCreature {
                                creature_id: source_id,
                                creature_card_id: card_id,
                                controller,
                                damaged_creature: *damaged_id,
                                amount: *amount,
                                description: desc,
                            };
                            if controller == active_player {
                                c.push_ap(trigger);
                            } else {
                                c.push_nap(trigger);
                            }
                        }
                    }
                }
            }
        }
        // Creature-to-player combat damage triggers.
        if let crate::events::DamageTarget::Player(damaged_player) = target {
            let source_id = *source;
            if let Some(obj) = state.get_object(source_id) {
                if obj.zone == Zone::Battlefield && state.is_creature(source_id, registry) {
                    let card_id = obj.card_id;
                    let controller = obj.controller;

                    // Source's own combat damage trigger (requires registered card).
                    if registry.get(card_id).is_some() {
                        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::CombatDamageToPlayer, false);
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
                                c.push_ap(trigger);
                            } else {
                                c.push_nap(trigger);
                            }
                        }
                    }

                    // Combat damage watchers and any-damage watchers.
                    // Includes self — cards like Rakish Heir watch their own damage.
                    let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield)
                        .map(|o| (o.id, o.card_id, o.controller))
                        .collect();
                    for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                        if registry.get(watcher_card_id).is_some() {
                            // AnyCombatDamageToPlayer watchers.
                            let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCombatDamageToPlayer, false);
                            if !desc.is_empty() {
                                let trigger = PendingTrigger::CombatDamageWatch {
                                    watcher_id,
                                    watcher_card_id,
                                    controller: watcher_controller,
                                    source_id,
                                    damaged_player: *damaged_player,
                                    amount: *amount,
                                    description: desc,
                                };
                                if watcher_controller == active_player {
                                    c.push_ap(trigger);
                                } else {
                                    c.push_nap(trigger);
                                }
                            }
                            // AnyDamageToPlayer watchers (combat damage is also damage).
                            let desc2 = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyDamageToPlayer, false);
                            // CR 603.2: the watcher's own condition on
                            // WHO dealt the damage and to WHOM.
                            if !desc2.is_empty()
                                && registry.get(watcher_card_id).is_some_and(|b|
                                    b.should_trigger_on_damage_to_player(state, watcher_id, source_id, *damaged_player, registry))
                            {
                                let trigger = PendingTrigger::DamageToPlayerWatch {
                                    watcher_id,
                                    watcher_card_id,
                                    controller: watcher_controller,
                                    source_id,
                                    damaged_player: *damaged_player,
                                    amount: *amount,
                                    description: desc2,
                                };
                                if watcher_controller == active_player {
                                    c.push_ap(trigger);
                                } else {
                                    c.push_nap(trigger);
                                }
                            }
                        }
                    }
                }
            }
        }
}

pub(super) fn noncombat_damage(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let active_player = c.active_player;
    let GameEvent::NonCombatDamageDealt { source, target: crate::events::DamageTarget::Player(damaged_player), amount } = event else { return };
        // AnyDamageToPlayer watchers for non-combat damage.
        {
            let source_id = *source;
            let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| (o.id, o.card_id, o.controller))
                .collect();
            for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                if registry.get(watcher_card_id).is_some() {
                    let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyDamageToPlayer, false);
                    if !desc.is_empty()
                        && registry.get(watcher_card_id).is_some_and(|b|
                            b.should_trigger_on_damage_to_player(state, watcher_id, source_id, *damaged_player, registry))
                    {
                        let trigger = PendingTrigger::DamageToPlayerWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            source_id,
                            damaged_player: *damaged_player,
                            amount: *amount,
                            description: desc,
                        };
                        if watcher_controller == active_player {
                            c.push_ap(trigger);
                        } else {
                            c.push_nap(trigger);
                        }
                    }
                }
            }
        }
}

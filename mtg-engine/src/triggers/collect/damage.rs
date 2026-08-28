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
                        c.emit(source_id, card_id, controller, desc,
                            TriggerEvent::CombatDamageToCreature {
                                damaged_creature: *damaged_id,
                                amount: *amount,
                            });
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
                        c.emit(source_id, card_id, controller, desc,
                            TriggerEvent::CombatDamageToPlayer {
                                damaged_player: *damaged_player,
                                amount: *amount,
                            });
                    }
                }

                // Combat damage watchers and any-damage watchers.
                // Includes self — cards like Rakish Heir watch their own damage.
                let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects_in_id_order().into_iter()
                    .filter(|o| o.zone == Zone::Battlefield)
                    .map(|o| (o.id, o.card_id, o.controller))
                    .collect();
                for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                    if registry.get(watcher_card_id).is_some() {
                        // AnyCombatDamageToPlayer watchers.
                        //
                        // CR 603.2: the watcher's own condition on WHO dealt
                        // the damage and to WHOM — the same gate its
                        // AnyDamageToPlayer twin below has always had. Without
                        // it, "whenever a creature deals combat damage to
                        // ENCHANTED PLAYER" and "whenever a VAMPIRE YOU
                        // CONTROL deals combat damage" put a trigger on the
                        // stack for every creature's combat damage to every
                        // player, and did nothing when it resolved. A trigger
                        // that should not have triggered is not free: it is a
                        // stack object with a priority window around it.
                        let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCombatDamageToPlayer, false);
                        if !desc.is_empty()
                            && registry.get(watcher_card_id).is_some_and(|b|
                                b.should_trigger_on_damage_to_player(state, watcher_id, source_id, *damaged_player, registry))
                        {
                            c.emit(watcher_id, watcher_card_id, watcher_controller, desc,
                                TriggerEvent::AnyCombatDamageToPlayer {
                                    dealer: source_id,
                                    damaged_player: *damaged_player,
                                    amount: *amount,
                                });
                        }
                        // AnyDamageToPlayer watchers (combat damage is also damage).
                        let desc2 = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyDamageToPlayer, false);
                        // CR 603.2: the watcher's own condition on
                        // WHO dealt the damage and to WHOM.
                        if !desc2.is_empty()
                            && registry.get(watcher_card_id).is_some_and(|b|
                                b.should_trigger_on_damage_to_player(state, watcher_id, source_id, *damaged_player, registry))
                        {
                            c.emit(watcher_id, watcher_card_id, watcher_controller, desc2,
                                TriggerEvent::AnyDamageToPlayer {
                                    dealer: source_id,
                                    damaged_player: *damaged_player,
                                    amount: *amount,
                                });
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
    let GameEvent::NonCombatDamageDealt { source, target: crate::events::DamageTarget::Player(damaged_player), amount } = event else { return };
    // AnyDamageToPlayer watchers for non-combat damage.
    {
        let source_id = *source;
        let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects_in_id_order().into_iter()
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
                    c.emit(watcher_id, watcher_card_id, watcher_controller, desc,
                        TriggerEvent::AnyDamageToPlayer {
                            dealer: source_id,
                            damaged_player: *damaged_player,
                            amount: *amount,
                        });
                }
            }
        }
    }
}

//! Triggers from attackers and blockers being declared.

use super::Collector;
use super::super::*;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::Zone;

pub(super) fn attackers_declared(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let GameEvent::AttackersDeclared { attackers } = event else { return };
    for (attacker_id, defending_player) in attackers {
        let (card_id, controller) = match state.get_object(*attacker_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
            _ => continue,
        };
        if registry.get(card_id).is_some() {
            let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::Attacks, false);
            if !desc.is_empty() {
                c.emit(*attacker_id, card_id, controller, desc, TriggerEvent::Attacks {
                    attacker: *attacker_id,
                    defending_player: *defending_player,
                });
            }
        }
        // Also check equipment/auras attached to the attacker.
        let attached: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.attached_to == Some(*attacker_id))
            .map(|o| (o.id, o.card_id, o.controller))
            .collect();
        for (eq_id, eq_card_id, eq_controller) in attached {
            if registry.get(eq_card_id).is_some() {
                let desc = trigger_description(registry, eq_card_id, &crate::cards::TriggerKind::Attacks, false);
                if !desc.is_empty() {
                    c.emit(eq_id, eq_card_id, eq_controller, desc, TriggerEvent::Attacks {
                        attacker: *attacker_id,
                        defending_player: *defending_player,
                    });
                }
            }
        }

        // Attack-watchers: notify permanents that care about any creature attacking.
        // Note: the attacker is NOT excluded — cards like Instigator Gang
        // ("attacking creatures you control get +1/+0") must see their own attack.
        let watchers: Vec<(ObjectId, CardId, PlayerId, bool)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| (o.id, o.card_id, o.controller, o.is_transformed))
            .collect();
        for (w_id, w_card_id, w_controller, w_transformed) in watchers {
            if registry.get(w_card_id).is_some() {
                let desc = trigger_description(registry, w_card_id, &crate::cards::TriggerKind::AnyCreatureAttacks, w_transformed);
                if !desc.is_empty() {
                    c.emit(w_id, w_card_id, w_controller, desc, TriggerEvent::CreatureAttacked {
                        attacker: *attacker_id,
                        attacker_controller: controller,
                    });
                }
            }
        }
    }
}

pub(super) fn blockers_declared(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let GameEvent::BlockersDeclared { assignments } = event else { return };
    for (blocker_id, attacker_id) in assignments {
        let (card_id, controller) = match state.get_object(*blocker_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
            _ => continue,
        };
        if let Some(b) = registry.get(card_id) {
            // CR 603.2: conditional Blocks triggers only fire
            // when the blocked creature matches.
            let desc = if b.should_trigger_on_blocks(state, *blocker_id, *attacker_id, registry) {
                trigger_description(registry, card_id, &crate::cards::TriggerKind::Blocks, false)
            } else {
                String::new()
            };
            if !desc.is_empty() {
                c.emit(*blocker_id, card_id, controller, desc,
                    TriggerEvent::Blocks { blocked_attacker: *attacker_id });
            }
        }
        // Also check equipment/auras attached to the blocker.
        let attached: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.attached_to == Some(*blocker_id))
            .map(|o| (o.id, o.card_id, o.controller))
            .collect();
        for (eq_id, eq_card_id, eq_controller) in attached {
            if let Some(b) = registry.get(eq_card_id) {
                let desc = if b.should_trigger_on_blocks(state, eq_id, *attacker_id, registry) {
                    trigger_description(registry, eq_card_id, &crate::cards::TriggerKind::Blocks, false)
                } else {
                    String::new()
                };
                if !desc.is_empty() {
                    c.emit(eq_id, eq_card_id, eq_controller, desc,
                        TriggerEvent::Blocks { blocked_attacker: *attacker_id });
                }
            }
        }

        // BecomesBlocked: the attacker gets a "becomes blocked" trigger.
        let (att_card_id, att_controller) = match state.get_object(*attacker_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
            _ => continue,
        };
        if let Some(b) = registry.get(att_card_id) {
            let desc = if b.should_trigger_on_becomes_blocked(state, *attacker_id, *blocker_id, registry) {
                trigger_description(registry, att_card_id, &crate::cards::TriggerKind::BecomesBlocked, false)
            } else {
                String::new()
            };
            if !desc.is_empty() {
                c.emit(*attacker_id, att_card_id, att_controller, desc,
                    TriggerEvent::BecomesBlocked { blocker_id: *blocker_id });
            }
        }
        // Check equipment/auras on the attacker for BecomesBlocked triggers.
        let att_attached: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.attached_to == Some(*attacker_id))
            .map(|o| (o.id, o.card_id, o.controller))
            .collect();
        for (eq_id, eq_card_id, eq_controller) in att_attached {
            if let Some(b) = registry.get(eq_card_id) {
                let desc = if b.should_trigger_on_becomes_blocked(state, eq_id, *blocker_id, registry) {
                    trigger_description(registry, eq_card_id, &crate::cards::TriggerKind::BecomesBlocked, false)
                } else {
                    String::new()
                };
                if !desc.is_empty() {
                    c.emit(eq_id, eq_card_id, eq_controller, desc,
                        TriggerEvent::BecomesBlocked { blocker_id: *blocker_id });
                }
            }
        }
    }
}

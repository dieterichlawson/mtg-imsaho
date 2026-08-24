//! Triggers from a step beginning, or a spell being cast.

use super::Collector;
use super::super::*;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::Zone;

pub(super) fn step_started(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let active_player = c.active_player;
    let GameEvent::StepStarted { step } = event else { return };
        // Drain delayed end-of-combat exile triggers (CR 603.7) into the
        // pending queue. These fire regardless of whether the source
        // permanent is still on the battlefield.
        if *step == crate::types::Step::EndCombat {
            let pending = std::mem::take(&mut state.end_of_combat_exiles);
            for entry in pending {
                let trigger = PendingTrigger::DelayedTokenExile {
                    target_id: entry.target_id,
                    source_card_id: entry.source_card_id,
                    controller: entry.controller,
                    description: entry.description,
                };
                if entry.controller == active_player {
                    c.push_ap(trigger);
                } else {
                    c.push_nap(trigger);
                }
            }
        }

        let trigger_kind = match step {
            crate::types::Step::Upkeep => Some(crate::cards::TriggerKind::Upkeep),
            crate::types::Step::EndCombat => Some(crate::cards::TriggerKind::EndCombat),
            crate::types::Step::EndStep => Some(crate::cards::TriggerKind::EndStep),
            _ => None,
        };
        if let Some(kind) = trigger_kind {
            let permanents: Vec<(ObjectId, CardId, PlayerId, bool)> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| (o.id, o.card_id, o.controller, o.is_transformed))
                .collect();
            for (obj_id, card_id, controller, is_transformed) in permanents {
                if let Some(behavior) = registry.get(card_id) {
                    let desc = face_trigger_description(registry, card_id, &kind, is_transformed);
                    if !desc.is_empty() {
                        // CR 603.2: the trigger event is a particular
                        // player's step beginning. Which player depends
                        // on the ability's scope.
                        let in_scope = match behavior.step_trigger_scope(&kind, is_transformed) {
                            crate::cards::TriggerScope::Each => true,
                            crate::cards::TriggerScope::Your => controller == active_player,
                            crate::cards::TriggerScope::AttachedPlayer => {
                                state.get_object(obj_id)
                                    .and_then(|o| o.attached_to_player)
                                    .is_some_and(|p| p == active_player)
                            }
                        };
                        if !in_scope {
                            continue;
                        }
                        // CR 603.4: intervening-if is checked at trigger
                        // time, so a false condition creates no stack entry.
                        if !behavior.should_trigger(state, obj_id, &kind, registry) {
                            continue;
                        }
                        let trigger = match kind {
                            crate::cards::TriggerKind::Upkeep => PendingTrigger::UpkeepTrigger {
                                object_id: obj_id,
                                card_id,
                                controller,
                                description: desc,
                                chosen_targets: Vec::new(),
                            },
                            crate::cards::TriggerKind::EndCombat => PendingTrigger::EndCombatTrigger {
                                object_id: obj_id,
                                card_id,
                                controller,
                                description: desc,
                            },
                            crate::cards::TriggerKind::EndStep => PendingTrigger::EndStepTrigger {
                                object_id: obj_id,
                                card_id,
                                controller,
                                description: desc,
                                chosen_targets: Vec::new(),
                            },
                            _ => unreachable!(),
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

pub(super) fn spell_cast(
    state: &mut GameState,
    _events: &[GameEvent],
    event: &GameEvent,
    registry: &CardRegistry,
    c: &mut Collector,
) {
    let active_player = c.active_player;
    let GameEvent::SpellCast { player: caster, object: spell_id } = event else { return };
        {
            let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| (o.id, o.card_id, o.controller))
                .collect();
            for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    // CR 603.2: only create the trigger when the
                    // watcher's full condition holds (caster / spell
                    // type restrictions), not for every spell cast.
                    if !behavior.should_trigger_on_spell_cast(state, watcher_id, *caster, *spell_id, registry) {
                        continue;
                    }
                    let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::SpellCast, false);
                    if !desc.is_empty() {
                        let trigger = PendingTrigger::SpellCastWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            caster: *caster,
                            spell_id: *spell_id,
                            description: desc,
                            chosen_targets: Vec::new(),
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

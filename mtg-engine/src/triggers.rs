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
        /// Last-known information captured before zone change clears battlefield state.
        dead_damaged_by: Vec<ObjectId>,
        dead_toughness: i32,
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
    /// A watcher observing another creature's combat damage to a player.
    CombatDamageWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        source_id: ObjectId,
        damaged_player: PlayerId,
        amount: u32,
        description: String,
    },
    /// A watcher observing any damage (combat or non-combat) to a player.
    DamageToPlayerWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        source_id: ObjectId,
        damaged_player: PlayerId,
        amount: u32,
        description: String,
    },
    /// A spell-cast watcher trigger.
    SpellCastWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        caster: PlayerId,
        spell_id: ObjectId,
        description: String,
    },
    /// An upkeep trigger on a permanent.
    UpkeepTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// An end-step trigger on a permanent.
    EndStepTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// A permanent leaving the battlefield trigger.
    LeftBattlefield {
        object_id: ObjectId,
        card_id: CardId,
        description: String,
    },
    /// A creature's "when this attacks" trigger.
    AttacksTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// A creature's "when this blocks" trigger.
    BlocksTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        blocked_attacker: ObjectId,
        description: String,
    },
    /// A watcher observing any creature attacking.
    AttackWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        attacker_id: ObjectId,
        attacker_controller: PlayerId,
        description: String,
    },
    /// A creature's "when this becomes blocked" trigger (attacker that gets blocked).
    BecomesBlockedTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        blocker_id: ObjectId,
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
            PendingTrigger::CombatDamageWatch { controller, .. } => *controller,
            PendingTrigger::DamageToPlayerWatch { controller, .. } => *controller,
            PendingTrigger::SpellCastWatch { controller, .. } => *controller,
            PendingTrigger::UpkeepTrigger { controller, .. } => *controller,
            PendingTrigger::EndStepTrigger { controller, .. } => *controller,
            PendingTrigger::LeftBattlefield { .. } => PlayerId(255),
            PendingTrigger::AttacksTrigger { controller, .. } => *controller,
            PendingTrigger::BlocksTrigger { controller, .. } => *controller,
            PendingTrigger::AttackWatch { controller, .. } => *controller,
            PendingTrigger::BecomesBlockedTrigger { controller, .. } => *controller,
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
            PendingTrigger::CombatDamageWatch { watcher_card_id, description, .. }
            | PendingTrigger::DamageToPlayerWatch { watcher_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s triggered ability", card_name(*watcher_card_id))
                } else {
                    format!("{}'s triggered ability ({})", card_name(*watcher_card_id), description)
                }
            }
            PendingTrigger::SpellCastWatch { watcher_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s triggered ability", card_name(*watcher_card_id))
                } else {
                    format!("{}'s triggered ability ({})", card_name(*watcher_card_id), description)
                }
            }
            PendingTrigger::UpkeepTrigger { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s upkeep trigger", card_name(*card_id))
                } else {
                    format!("{}'s upkeep trigger ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::EndStepTrigger { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s end step trigger", card_name(*card_id))
                } else {
                    format!("{}'s end step trigger ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::LeftBattlefield { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s LTB trigger", card_name(*card_id))
                } else {
                    format!("{}'s LTB trigger ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::AttacksTrigger { card_id, description, .. }
            | PendingTrigger::AttackWatch { watcher_card_id: card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s attack trigger", card_name(*card_id))
                } else {
                    format!("{}'s attack trigger ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::BlocksTrigger { card_id, description, .. }
            | PendingTrigger::BecomesBlockedTrigger { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s block trigger", card_name(*card_id))
                } else {
                    format!("{}'s block trigger ({})", card_name(*card_id), description)
                }
            }
        }
    }
}

/// Look up the description for a trigger from the card's TriggeredAbilityDef.
/// For transformed DFCs, also check the back face's triggered abilities.
fn trigger_description(registry: &CardRegistry, card_id: CardId, kind: &crate::cards::TriggerKind, is_transformed: bool) -> String {
    if let Some(behavior) = registry.get(card_id) {
        // Check front face triggers.
        if let Some(t) = behavior.card_data().triggered_abilities.iter().find(|t| &t.kind == kind) {
            return t.description.clone();
        }
        // For transformed DFCs, also check back face triggers.
        if is_transformed {
            if let Some(back) = behavior.back_face_data() {
                if let Some(t) = back.triggered_abilities.iter().find(|t| &t.kind == kind) {
                    return t.description.clone();
                }
            }
        }
    }
    String::new()
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
                    let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::EntersBattlefield, false);
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
                            let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCreatureEnters, false);
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
            GameEvent::CreatureDied { object, card_id, controller, damaged_by, last_known_toughness } => {
                let dead_id = *object;
                let dead_card_id = *card_id;
                let dead_controller = *controller;
                let dead_damaged_by = damaged_by.clone();
                let dead_toughness = *last_known_toughness;

                // 1. Self-dies trigger.
                if registry.get(dead_card_id).is_some() {
                    let desc = trigger_description(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies, false);
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
                        let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCreatureDies, false);
                        let trigger = PendingTrigger::DeathWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            dead_id,
                            dead_controller,
                            dead_damaged_by: dead_damaged_by.clone(),
                            dead_toughness,
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
                    let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::LeavesBattlefield, false);
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
                                        ap_triggers.push(trigger);
                                    } else {
                                        nap_triggers.push(trigger);
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
                                            ap_triggers.push(trigger);
                                        } else {
                                            nap_triggers.push(trigger);
                                        }
                                    }
                                    // AnyDamageToPlayer watchers (combat damage is also damage).
                                    let desc2 = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyDamageToPlayer, false);
                                    if !desc2.is_empty() {
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
            }
            GameEvent::NonCombatDamageDealt { source, target, amount } => {
                // AnyDamageToPlayer watchers for non-combat damage.
                if let crate::events::DamageTarget::Player(damaged_player) = target {
                    let source_id = *source;
                    let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield)
                        .map(|o| (o.id, o.card_id, o.controller))
                        .collect();
                    for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                        if registry.get(watcher_card_id).is_some() {
                            let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyDamageToPlayer, false);
                            if !desc.is_empty() {
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
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
                                }
                            }
                        }
                    }
                }
            }
            GameEvent::StepStarted { step } => {
                let trigger_kind = match step {
                    crate::types::Step::Upkeep => Some(crate::cards::TriggerKind::Upkeep),
                    crate::types::Step::EndStep => Some(crate::cards::TriggerKind::EndStep),
                    _ => None,
                };
                if let Some(kind) = trigger_kind {
                    let permanents: Vec<(ObjectId, CardId, PlayerId, bool)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield)
                        .map(|o| (o.id, o.card_id, o.controller, o.is_transformed))
                        .collect();
                    for (obj_id, card_id, controller, is_transformed) in permanents {
                        if registry.get(card_id).is_some() {
                            let desc = trigger_description(registry, card_id, &kind, is_transformed);
                            if !desc.is_empty() {
                                let trigger = match kind {
                                    crate::cards::TriggerKind::Upkeep => PendingTrigger::UpkeepTrigger {
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
                                    },
                                    _ => unreachable!(),
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
            GameEvent::SpellCast { player: caster, object: spell_id } => {
                // Check if the spell is an instant or sorcery.
                let is_instant_sorcery = state.get_object(*spell_id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .map(|d| d.card_types.iter().any(|ct| matches!(ct, crate::types::CardType::Instant | crate::types::CardType::Sorcery)))
                    .unwrap_or(false);
                if is_instant_sorcery {
                    let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield)
                        .map(|o| (o.id, o.card_id, o.controller))
                        .collect();
                    for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                        if registry.get(watcher_card_id).is_some() {
                            let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::SpellCast, false);
                            if !desc.is_empty() {
                                let trigger = PendingTrigger::SpellCastWatch {
                                    watcher_id,
                                    watcher_card_id,
                                    controller: watcher_controller,
                                    caster: *caster,
                                    spell_id: *spell_id,
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
            GameEvent::AttackersDeclared { attackers } => {
                for (attacker_id, _defending_player) in attackers {
                    let (card_id, controller) = match state.get_object(*attacker_id) {
                        Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
                        _ => continue,
                    };
                    if registry.get(card_id).is_some() {
                        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::Attacks, false);
                        if !desc.is_empty() {
                            let trigger = PendingTrigger::AttacksTrigger {
                                object_id: *attacker_id,
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
                                let trigger = PendingTrigger::AttacksTrigger {
                                    object_id: eq_id,
                                    card_id: eq_card_id,
                                    controller: eq_controller,
                                    description: desc,
                                };
                                if eq_controller == active_player {
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
                                }
                            }
                        }
                    }

                    // Attack-watchers: notify other permanents (like AnyCreatureDies watchers).
                    let watchers: Vec<(ObjectId, CardId, PlayerId, bool)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield && o.id != *attacker_id)
                        .map(|o| (o.id, o.card_id, o.controller, o.is_transformed))
                        .collect();
                    for (w_id, w_card_id, w_controller, w_transformed) in watchers {
                        if registry.get(w_card_id).is_some() {
                            let desc = trigger_description(registry, w_card_id, &crate::cards::TriggerKind::AnyCreatureAttacks, w_transformed);
                            if !desc.is_empty() {
                                let trigger = PendingTrigger::AttackWatch {
                                    watcher_id: w_id,
                                    watcher_card_id: w_card_id,
                                    controller: w_controller,
                                    attacker_id: *attacker_id,
                                    attacker_controller: controller,
                                    description: desc,
                                };
                                if w_controller == active_player {
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
                                }
                            }
                        }
                    }
                }
            }
            GameEvent::BlockersDeclared { assignments } => {
                for (blocker_id, attacker_id) in assignments {
                    let (card_id, controller) = match state.get_object(*blocker_id) {
                        Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
                        _ => continue,
                    };
                    if registry.get(card_id).is_some() {
                        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::Blocks, false);
                        if !desc.is_empty() {
                            let trigger = PendingTrigger::BlocksTrigger {
                                object_id: *blocker_id,
                                card_id,
                                controller,
                                blocked_attacker: *attacker_id,
                                description: desc,
                            };
                            if controller == active_player {
                                ap_triggers.push(trigger);
                            } else {
                                nap_triggers.push(trigger);
                            }
                        }
                    }
                    // Also check equipment/auras attached to the blocker.
                    let attached: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield && o.attached_to == Some(*blocker_id))
                        .map(|o| (o.id, o.card_id, o.controller))
                        .collect();
                    for (eq_id, eq_card_id, eq_controller) in attached {
                        if registry.get(eq_card_id).is_some() {
                            let desc = trigger_description(registry, eq_card_id, &crate::cards::TriggerKind::Blocks, false);
                            if !desc.is_empty() {
                                let trigger = PendingTrigger::BlocksTrigger {
                                    object_id: eq_id,
                                    card_id: eq_card_id,
                                    controller: eq_controller,
                                    blocked_attacker: *attacker_id,
                                    description: desc,
                                };
                                if eq_controller == active_player {
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
                                }
                            }
                        }
                    }

                    // BecomesBlocked: the attacker gets a "becomes blocked" trigger.
                    let (att_card_id, att_controller) = match state.get_object(*attacker_id) {
                        Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.controller),
                        _ => continue,
                    };
                    if registry.get(att_card_id).is_some() {
                        let desc = trigger_description(registry, att_card_id, &crate::cards::TriggerKind::BecomesBlocked, false);
                        if !desc.is_empty() {
                            let trigger = PendingTrigger::BecomesBlockedTrigger {
                                object_id: *attacker_id,
                                card_id: att_card_id,
                                controller: att_controller,
                                blocker_id: *blocker_id,
                                description: desc,
                            };
                            if att_controller == active_player {
                                ap_triggers.push(trigger);
                            } else {
                                nap_triggers.push(trigger);
                            }
                        }
                    }
                    // Check equipment/auras on the attacker for BecomesBlocked triggers.
                    let att_attached: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                        .filter(|o| o.zone == Zone::Battlefield && o.attached_to == Some(*attacker_id))
                        .map(|o| (o.id, o.card_id, o.controller))
                        .collect();
                    for (eq_id, eq_card_id, eq_controller) in att_attached {
                        if registry.get(eq_card_id).is_some() {
                            let desc = trigger_description(registry, eq_card_id, &crate::cards::TriggerKind::BecomesBlocked, false);
                            if !desc.is_empty() {
                                let trigger = PendingTrigger::BecomesBlockedTrigger {
                                    object_id: eq_id,
                                    card_id: eq_card_id,
                                    controller: eq_controller,
                                    blocker_id: *blocker_id,
                                    description: desc,
                                };
                                if eq_controller == active_player {
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
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
        PendingTrigger::DeathWatch { watcher_id, watcher_card_id, dead_id, dead_controller, dead_damaged_by, dead_toughness, .. } => {
            // Verify the watcher is still on the battlefield.
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_creature_dies(state, watcher_id, dead_id, dead_controller, &dead_damaged_by, dead_toughness, registry);
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
            if let Some(behavior) = registry.get(creature_card_id) {
                behavior.on_combat_damage_to_player(state, creature_id, damaged_player, amount, registry);
            }
        }
        PendingTrigger::CombatDamageWatch { watcher_id, watcher_card_id, source_id, damaged_player, amount, .. } => {
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_combat_damage_to_player(state, watcher_id, source_id, damaged_player, amount, registry);
                }
            }
        }
        PendingTrigger::DamageToPlayerWatch { watcher_id, watcher_card_id, source_id, damaged_player, amount, .. } => {
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_damage_to_player(state, watcher_id, source_id, damaged_player, amount, registry);
                }
            }
        }
        PendingTrigger::UpkeepTrigger { object_id, card_id, .. } => {
            if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_upkeep(state, object_id, registry);
                }
            }
        }
        PendingTrigger::EndStepTrigger { object_id, card_id, .. } => {
            if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_end_step(state, object_id, registry);
                }
            }
        }
        PendingTrigger::SpellCastWatch { watcher_id, watcher_card_id, caster, spell_id, .. } => {
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_spell_cast(state, watcher_id, caster, spell_id, registry);
                }
            }
        }
        PendingTrigger::LeftBattlefield { object_id, card_id, .. } => {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_leave_battlefield(state, object_id, registry);
            }
        }
        PendingTrigger::AttacksTrigger { object_id, card_id, .. } => {
            if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_attacks(state, object_id, registry);
                }
            }
        }
        PendingTrigger::BlocksTrigger { object_id, card_id, blocked_attacker, .. } => {
            if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_blocks(state, object_id, blocked_attacker, registry);
                }
            }
        }
        PendingTrigger::AttackWatch { watcher_id, watcher_card_id, attacker_id, attacker_controller, .. } => {
            if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_creature_attacks(state, watcher_id, attacker_id, attacker_controller, registry);
                }
            }
        }
        PendingTrigger::BecomesBlockedTrigger { object_id, card_id, blocker_id, .. } => {
            if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_becomes_blocked(state, object_id, blocker_id, registry);
                }
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

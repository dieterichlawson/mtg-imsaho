use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameState, StackEntry};
use crate::types::Zone;

/// A triggered ability that has been collected but not yet resolved.
/// These are placed on `pending_triggers` in APNAP order, then resolved
/// LIFO (non-active player's triggers resolve first).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PendingTrigger {
    /// A creature's own "when this dies" trigger.
    SelfDies {
        dead_id: ObjectId,
        dead_card_id: CardId,
        controller: PlayerId,
        description: String,
        /// CR 603.3d: targets chosen as the trigger goes on the stack. Empty
        /// for untargeted triggers.
        #[serde(default)]
        chosen_targets: Vec<Target>,
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
        dead_is_token: bool,
        description: String,
        #[serde(default)]
        chosen_targets: Vec<Target>,
    },
    /// A creature entering the battlefield trigger.
    EnteredBattlefield {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
        #[serde(default)]
        chosen_targets: Vec<Target>,
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
        #[serde(default)]
        chosen_targets: Vec<Target>,
    },
    /// An end-of-combat trigger on a permanent.
    EndCombatTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// An upkeep trigger on a permanent.
    UpkeepTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
        #[serde(default)]
        chosen_targets: Vec<Target>,
    },
    /// An end-step trigger on a permanent.
    EndStepTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
        #[serde(default)]
        chosen_targets: Vec<Target>,
    },
    /// A permanent leaving the battlefield trigger. Per CR 603.10c, the
    /// controller is the player who controlled the permanent immediately
    /// before it left the battlefield.
    LeftBattlefield {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// A creature's "when this attacks" trigger.
    AttacksTrigger {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
        #[serde(default)]
        chosen_targets: Vec<Target>,
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
    /// A creature's "when this deals combat damage to a creature" trigger.
    CombatDamageToCreature {
        creature_id: ObjectId,
        creature_card_id: CardId,
        controller: PlayerId,
        damaged_creature: ObjectId,
        amount: u32,
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
    /// A state-triggered ability (CR 603.8). These are checked during SBA processing
    /// and go on the stack like normal triggered abilities. The trigger won't fire again
    /// while it's already on the stack.
    StateTriggered {
        object_id: ObjectId,
        card_id: CardId,
        controller: PlayerId,
        description: String,
    },
    /// A watcher observing a creature card being milled from an opponent's library.
    CreatureCardMilledWatch {
        watcher_id: ObjectId,
        watcher_card_id: CardId,
        controller: PlayerId,
        milled_object: ObjectId,
        milled_player: PlayerId,
        description: String,
    },
}

impl PendingTrigger {
    /// The player who controls this trigger.
    #[must_use]
    pub fn controller(&self) -> PlayerId {
        match self {
            PendingTrigger::SelfDies { controller, .. }
            | PendingTrigger::DeathWatch { controller, .. }
            | PendingTrigger::EnteredBattlefield { controller, .. }
            | PendingTrigger::EnterWatch { controller, .. }
            | PendingTrigger::CombatDamageToPlayer { controller, .. }
            | PendingTrigger::CombatDamageWatch { controller, .. }
            | PendingTrigger::DamageToPlayerWatch { controller, .. }
            | PendingTrigger::SpellCastWatch { controller, .. }
            | PendingTrigger::EndCombatTrigger { controller, .. }
            | PendingTrigger::UpkeepTrigger { controller, .. }
            | PendingTrigger::EndStepTrigger { controller, .. }
            | PendingTrigger::LeftBattlefield { controller, .. }
            | PendingTrigger::AttacksTrigger { controller, .. }
            | PendingTrigger::BlocksTrigger { controller, .. }
            | PendingTrigger::AttackWatch { controller, .. }
            | PendingTrigger::CombatDamageToCreature { controller, .. }
            | PendingTrigger::BecomesBlockedTrigger { controller, .. }
            | PendingTrigger::StateTriggered { controller, .. }
            | PendingTrigger::CreatureCardMilledWatch { controller, .. } => *controller,
        }
    }

    /// The source object that the trigger fires from. Used to look up the
    /// trigger's target requirement and as the "source" for target validation.
    /// Returns None for triggers without a meaningful source object.
    #[must_use]
    pub fn source_object(&self) -> Option<ObjectId> {
        match self {
            PendingTrigger::SelfDies { dead_id, .. } => Some(*dead_id),
            PendingTrigger::DeathWatch { watcher_id, .. }
            | PendingTrigger::EnterWatch { watcher_id, .. }
            | PendingTrigger::CombatDamageWatch { watcher_id, .. }
            | PendingTrigger::DamageToPlayerWatch { watcher_id, .. }
            | PendingTrigger::SpellCastWatch { watcher_id, .. }
            | PendingTrigger::AttackWatch { watcher_id, .. }
            | PendingTrigger::CreatureCardMilledWatch { watcher_id, .. } => Some(*watcher_id),
            PendingTrigger::EnteredBattlefield { object_id, .. }
            | PendingTrigger::EndCombatTrigger { object_id, .. }
            | PendingTrigger::UpkeepTrigger { object_id, .. }
            | PendingTrigger::EndStepTrigger { object_id, .. }
            | PendingTrigger::LeftBattlefield { object_id, .. }
            | PendingTrigger::AttacksTrigger { object_id, .. }
            | PendingTrigger::BlocksTrigger { object_id, .. }
            | PendingTrigger::BecomesBlockedTrigger { object_id, .. }
            | PendingTrigger::StateTriggered { object_id, .. } => Some(*object_id),
            PendingTrigger::CombatDamageToPlayer { creature_id, .. }
            | PendingTrigger::CombatDamageToCreature { creature_id, .. } => Some(*creature_id),
        }
    }

    /// The card id of the behavior that owns this trigger. Used to look up
    /// the `TriggeredAbilityDef`.
    #[must_use]
    pub fn behavior_card_id(&self) -> CardId {
        match self {
            PendingTrigger::SelfDies { dead_card_id, .. } => *dead_card_id,
            PendingTrigger::DeathWatch { watcher_card_id, .. }
            | PendingTrigger::EnterWatch { watcher_card_id, .. }
            | PendingTrigger::CombatDamageWatch { watcher_card_id, .. }
            | PendingTrigger::DamageToPlayerWatch { watcher_card_id, .. }
            | PendingTrigger::SpellCastWatch { watcher_card_id, .. }
            | PendingTrigger::AttackWatch { watcher_card_id, .. }
            | PendingTrigger::CreatureCardMilledWatch { watcher_card_id, .. } => *watcher_card_id,
            PendingTrigger::EnteredBattlefield { card_id, .. }
            | PendingTrigger::EndCombatTrigger { card_id, .. }
            | PendingTrigger::UpkeepTrigger { card_id, .. }
            | PendingTrigger::EndStepTrigger { card_id, .. }
            | PendingTrigger::LeftBattlefield { card_id, .. }
            | PendingTrigger::AttacksTrigger { card_id, .. }
            | PendingTrigger::BlocksTrigger { card_id, .. }
            | PendingTrigger::BecomesBlockedTrigger { card_id, .. }
            | PendingTrigger::StateTriggered { card_id, .. } => *card_id,
            PendingTrigger::CombatDamageToPlayer { creature_card_id, .. }
            | PendingTrigger::CombatDamageToCreature { creature_card_id, .. } => *creature_card_id,
        }
    }

    /// The trigger kind, used to look up the matching `TriggeredAbilityDef`.
    /// Returns `None` for triggers without a corresponding `TriggerKind`
    /// (currently just `StateTriggered`, which has its own dispatch).
    #[must_use]
    pub fn kind(&self) -> Option<crate::cards::TriggerKind> {
        use crate::cards::TriggerKind;
        Some(match self {
            PendingTrigger::SelfDies { .. } => TriggerKind::SelfDies,
            PendingTrigger::DeathWatch { .. } => TriggerKind::AnyCreatureDies,
            PendingTrigger::EnteredBattlefield { .. } => TriggerKind::EntersBattlefield,
            PendingTrigger::EnterWatch { .. } => TriggerKind::AnyCreatureEnters,
            PendingTrigger::CombatDamageToPlayer { .. } => TriggerKind::CombatDamageToPlayer,
            PendingTrigger::CombatDamageWatch { .. } => TriggerKind::AnyCombatDamageToPlayer,
            PendingTrigger::DamageToPlayerWatch { .. } => TriggerKind::AnyDamageToPlayer,
            PendingTrigger::SpellCastWatch { .. } => TriggerKind::SpellCast,
            PendingTrigger::EndCombatTrigger { .. } => TriggerKind::EndCombat,
            PendingTrigger::UpkeepTrigger { .. } => TriggerKind::Upkeep,
            PendingTrigger::EndStepTrigger { .. } => TriggerKind::EndStep,
            PendingTrigger::LeftBattlefield { .. } => TriggerKind::LeavesBattlefield,
            PendingTrigger::AttacksTrigger { .. } => TriggerKind::Attacks,
            PendingTrigger::BlocksTrigger { .. } => TriggerKind::Blocks,
            PendingTrigger::AttackWatch { .. } => TriggerKind::AnyCreatureAttacks,
            PendingTrigger::CombatDamageToCreature { .. } => TriggerKind::DealsCombatDamageToCreature,
            PendingTrigger::BecomesBlockedTrigger { .. } => TriggerKind::BecomesBlocked,
            PendingTrigger::CreatureCardMilledWatch { .. } => TriggerKind::CreatureCardMilled,
            PendingTrigger::StateTriggered { .. } => return None,
        })
    }

    /// Mutate the trigger's `chosen_targets` field. Returns false if this
    /// variant doesn't carry chosen targets.
    pub fn set_chosen_targets(&mut self, targets: Vec<Target>) -> bool {
        match self {
            PendingTrigger::SelfDies { chosen_targets, .. }
            | PendingTrigger::DeathWatch { chosen_targets, .. }
            | PendingTrigger::EnteredBattlefield { chosen_targets, .. }
            | PendingTrigger::SpellCastWatch { chosen_targets, .. }
            | PendingTrigger::UpkeepTrigger { chosen_targets, .. }
            | PendingTrigger::EndStepTrigger { chosen_targets, .. }
            | PendingTrigger::AttacksTrigger { chosen_targets, .. } => {
                *chosen_targets = targets;
                true
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn chosen_targets(&self) -> &[Target] {
        match self {
            PendingTrigger::SelfDies { chosen_targets, .. }
            | PendingTrigger::DeathWatch { chosen_targets, .. }
            | PendingTrigger::EnteredBattlefield { chosen_targets, .. }
            | PendingTrigger::SpellCastWatch { chosen_targets, .. }
            | PendingTrigger::UpkeepTrigger { chosen_targets, .. }
            | PendingTrigger::EndStepTrigger { chosen_targets, .. }
            | PendingTrigger::AttacksTrigger { chosen_targets, .. } => chosen_targets,
            _ => &[],
        }
    }

    /// Display name for the stack view, including what the trigger does.
    /// Uses the object's current transform state to show the correct face name.
    #[must_use]
    pub fn display_name(&self, registry: &crate::cards::CardRegistry) -> String {
        self.display_name_with_state(registry, None)
    }

    /// Display name with optional game state for transform-aware name lookup.
    #[must_use]
    pub fn display_name_with_state(&self, registry: &crate::cards::CardRegistry, state: Option<&crate::state::GameState>) -> String {
        let face_name = |card_id: CardId, object_id: Option<ObjectId>| {
            // Check if the object is transformed and use back-face name if so.
            let is_transformed = object_id
                .and_then(|oid| state.and_then(|s| s.get_object(oid)))
                .is_some_and(|o| o.is_transformed);
            if is_transformed {
                if let Some(behavior) = registry.get(card_id) {
                    if let Some(back) = behavior.back_face_data() {
                        return back.name;
                    }
                }
            }
            registry.card_data(card_id).map_or_else(|| "Unknown".into(), |d| d.name)
        };
        let card_name = |card_id: CardId| {
            face_name(card_id, None)
        };
        match self {
            PendingTrigger::SelfDies { dead_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s dies trigger", card_name(*dead_card_id))
                } else {
                    format!("{}'s dies trigger ({})", card_name(*dead_card_id), description)
                }
            }
            PendingTrigger::DeathWatch { watcher_card_id, description, .. }
            | PendingTrigger::EnterWatch { watcher_card_id, description, .. }
            | PendingTrigger::CombatDamageWatch { watcher_card_id, description, .. }
            | PendingTrigger::DamageToPlayerWatch { watcher_card_id, description, .. }
            | PendingTrigger::SpellCastWatch { watcher_card_id, description, .. } => {
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
            PendingTrigger::CombatDamageToPlayer { creature_card_id, description, .. }
            | PendingTrigger::CombatDamageToCreature { creature_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s combat damage trigger", card_name(*creature_card_id))
                } else {
                    format!("{}'s combat damage trigger ({})", card_name(*creature_card_id), description)
                }
            }
            PendingTrigger::EndCombatTrigger { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s end of combat trigger", card_name(*card_id))
                } else {
                    format!("{}'s end of combat trigger ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::UpkeepTrigger { object_id, card_id, description, .. } => {
                let name = face_name(*card_id, Some(*object_id));
                if description.is_empty() {
                    format!("{name}'s upkeep trigger")
                } else {
                    format!("{name}'s upkeep trigger ({description})")
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
            PendingTrigger::StateTriggered { card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s state-triggered ability", card_name(*card_id))
                } else {
                    format!("{}'s state-triggered ability ({})", card_name(*card_id), description)
                }
            }
            PendingTrigger::CreatureCardMilledWatch { watcher_card_id, description, .. } => {
                if description.is_empty() {
                    format!("{}'s mill-watcher trigger", card_name(*watcher_card_id))
                } else {
                    format!("{}'s mill-watcher trigger ({})", card_name(*watcher_card_id), description)
                }
            }
        }
    }
}

/// Look up the description for a trigger from the card's `TriggeredAbilityDef`.
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

/// True if this card has a `TriggeredAbilityDef` for `kind` on either face.
/// Used to gate empty trigger creation — only cards with a declared ability of
/// this kind should put a trigger on the stack. Checks both front and back
/// face because (a) by the time SelfDies/LeftBattlefield is collected the
/// creature may have already been reset to front face by `move_object`, and
/// (b) either face having the ability means the card legitimately cares about
/// the event.
fn card_has_trigger(registry: &CardRegistry, card_id: CardId, kind: &crate::cards::TriggerKind) -> bool {
    let Some(behavior) = registry.get(card_id) else { return false };
    if behavior.card_data().triggered_abilities.iter().any(|t| &t.kind == kind) {
        return true;
    }
    if let Some(back) = behavior.back_face_data() {
        if back.triggered_abilities.iter().any(|t| &t.kind == kind) {
            return true;
        }
    }
    false
}

/// Look up a trigger description from the currently visible face only.
/// Unlike `trigger_description`, this does not fall back to the back face
/// for transformed cards — it reads the face that matches `is_transformed`.
/// Used where a per-face ability (e.g. Homicidal Brute's end-step trigger)
/// must not fire when the card is on the other face.
fn face_trigger_description(registry: &CardRegistry, card_id: CardId, kind: &crate::cards::TriggerKind, is_transformed: bool) -> String {
    let Some(behavior) = registry.get(card_id) else { return String::new() };
    let abilities = if is_transformed {
        behavior.back_face_data().map(|d| d.triggered_abilities).unwrap_or_default()
    } else {
        behavior.card_data().triggered_abilities
    };
    abilities.iter()
        .find(|t| &t.kind == kind)
        .map(|t| t.description.clone())
        .unwrap_or_default()
}

/// Collect triggered abilities from events and add them to the stack
/// in APNAP order (active player first on bottom, non-active player on top).
///
/// Does NOT resolve them — the game loop resolves them one at a time,
/// giving players priority between each.
pub fn collect_triggers(state: &mut GameState, registry: &CardRegistry) -> bool {
    let events = state.events.clone();
    let start = state.trigger_event_index;
    let active_player = state.active_player;

    let mut ap_triggers: Vec<PendingTrigger> = Vec::new();
    let mut nap_triggers: Vec<PendingTrigger> = Vec::new();

    for event in events.iter().skip(start) {
        match event {
            GameEvent::EnteredBattlefield { object, .. } => {
                // Per MTG rules, ETB triggers fire even if the source has since
                // left the battlefield. The trigger was created by the entering event.
                let (card_id, controller) = match state.get_object(*object) {
                    Some(o) => (o.card_id, o.controller),
                    _ => continue,
                };
                // Only collect if the card actually has an ETB handler.
                // Cards without one (basic lands, vanilla creatures) shouldn't put a
                // trigger on the stack — per MTG rules, only declared triggered abilities
                // create stack entries.
                if let Some(behavior) = registry.get(card_id) {
                    if behavior.has_etb_handler() {
                        let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::EntersBattlefield, false);
                        let trigger = PendingTrigger::EnteredBattlefield {
                            object_id: *object,
                            card_id,
                            controller,
                            description: desc,
                            chosen_targets: Vec::new(),
                        };
                        if controller == active_player {
                            ap_triggers.push(trigger);
                        } else {
                            nap_triggers.push(trigger);
                        }
                    }
                }

                // ETB-watch: notify other permanents (and graveyard cards like Dearly Departed)
                // that a creature entered. Only collect if the watcher's zone matches
                // the trigger's allowed zones (via CardBehavior::trigger_zones).
                if state.get_object(*object).is_some_and(|o| o.power.is_some()) {
                    let watchers: Vec<(ObjectId, CardId, PlayerId, Zone)> = state.objects.values()
                        .filter(|o| (o.zone == Zone::Battlefield || o.zone == Zone::Graveyard) && o.id != *object)
                        .map(|o| (o.id, o.card_id, o.controller, o.zone))
                        .collect();
                    let trigger_kind = crate::cards::TriggerKind::AnyCreatureEnters;
                    for (watcher_id, watcher_card_id, watcher_controller, watcher_zone) in watchers {
                        if let Some(behavior) = registry.get(watcher_card_id) {
                            let has_trigger = behavior.card_data().triggered_abilities.iter()
                                .any(|t| t.kind == trigger_kind);
                            if has_trigger && behavior.trigger_zones(&trigger_kind).contains(&watcher_zone) {
                                let desc = trigger_description(registry, watcher_card_id, &trigger_kind, false);
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
            GameEvent::CreatureDied { object, card_id, controller, damaged_by, last_known_toughness, is_token } => {
                let dead_id = *object;
                let dead_card_id = *card_id;
                let dead_controller = *controller;
                let dead_damaged_by = damaged_by.clone();
                let dead_toughness = *last_known_toughness;
                let dead_is_token = *is_token;

                // 1. Self-dies trigger. Only fire if the card actually has a
                // SelfDies TriggeredAbilityDef — vanilla creatures and creatures
                // with only watcher/ETB/activated abilities must not pollute the
                // stack with empty triggers.
                if card_has_trigger(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies) {
                    let desc = trigger_description(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies, false);
                    let trigger = PendingTrigger::SelfDies {
                        dead_id,
                        dead_card_id,
                        controller: dead_controller,
                        description: desc,
                        chosen_targets: Vec::new(),
                    };
                    if dead_controller == active_player {
                        ap_triggers.push(trigger);
                    } else {
                        nap_triggers.push(trigger);
                    }
                }

                // 2. Death-watch: collect triggers from permanents on the battlefield,
                // plus creatures that died in the same event batch (they were on the
                // battlefield when the simultaneous deaths occurred).
                let simultaneously_dead: Vec<ObjectId> = events.iter().filter_map(|e| {
                    if let GameEvent::CreatureDied { object, .. } = e {
                        Some(*object)
                    } else {
                        None
                    }
                }).collect();
                let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
                    .filter(|o| o.id != dead_id &&
                        (o.zone == Zone::Battlefield || simultaneously_dead.contains(&o.id)))
                    .map(|o| (o.id, o.card_id, o.controller))
                    .collect();
                for (watcher_id, watcher_card_id, watcher_controller) in watchers {
                    // Only create death-watch triggers for cards that actually have
                    // an AnyCreatureDies triggered ability.
                    let has_death_trigger = registry.get(watcher_card_id)
                        .is_some_and(|b| b.card_data().triggered_abilities.iter()
                            .any(|t| t.kind == crate::cards::TriggerKind::AnyCreatureDies));
                    if has_death_trigger {
                        let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyCreatureDies, false);
                        let trigger = PendingTrigger::DeathWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            dead_id,
                            dead_controller,
                            dead_damaged_by: dead_damaged_by.clone(),
                            dead_toughness,
                            dead_is_token,
                            description: desc,
                            chosen_targets: Vec::new(),
                        };
                        if watcher_controller == active_player {
                            ap_triggers.push(trigger);
                        } else {
                            nap_triggers.push(trigger);
                        }
                    }
                }
            }
            GameEvent::LeftBattlefield { object, last_controller, .. } => {
                let (card_id,) = match state.get_object(*object) {
                    Some(o) => (o.card_id,),
                    None => continue,
                };
                // Only fire LTB triggers for cards that actually have a
                // LeavesBattlefield TriggeredAbilityDef. Vanilla creatures,
                // auras without LTB clauses (Bonds of Faith, Dead Weight),
                // and other non-LTB permanents must not pollute the stack.
                if card_has_trigger(registry, card_id, &crate::cards::TriggerKind::LeavesBattlefield) {
                    let desc = trigger_description(registry, card_id, &crate::cards::TriggerKind::LeavesBattlefield, false);
                    let trigger = PendingTrigger::LeftBattlefield {
                        object_id: *object,
                        card_id,
                        controller: *last_controller,
                        description: desc,
                    };
                    if *last_controller == active_player {
                        ap_triggers.push(trigger);
                    } else {
                        nap_triggers.push(trigger);
                    }
                }
            }
            GameEvent::CombatDamageDealt { source, target, amount } => {
                // Creature-to-creature combat damage triggers.
                if let crate::events::DamageTarget::Object(damaged_id) = target {
                    let source_id = *source;
                    if let Some(obj) = state.get_object(source_id) {
                        if obj.zone == Zone::Battlefield && obj.power.is_some() {
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
                                        ap_triggers.push(trigger);
                                    } else {
                                        nap_triggers.push(trigger);
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
            GameEvent::NonCombatDamageDealt { source, target: crate::events::DamageTarget::Player(damaged_player), amount } => {
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
                        if registry.get(card_id).is_some() {
                            // Use face-aware lookup: only fire if the currently visible
                            // face has this step trigger. A front-face card must not
                            // fire a trigger defined only on the back face (Bug E).
                            let desc = face_trigger_description(registry, card_id, &kind, is_transformed);
                            if !desc.is_empty() {
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
                // Dispatch SpellCast triggers for ALL spell types (not just instant/sorcery).
                // Individual card handlers can filter by spell type if needed.
                {
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
                                    chosen_targets: Vec::new(),
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
                                chosen_targets: Vec::new(),
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
                                    chosen_targets: Vec::new(),
                                };
                                if eq_controller == active_player {
                                    ap_triggers.push(trigger);
                                } else {
                                    nap_triggers.push(trigger);
                                }
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
            GameEvent::CreatureCardMilled { object, milled_player } => {
                let milled_obj = *object;
                let milled_player = *milled_player;
                // Find watchers on the battlefield with CreatureCardMilled triggers.
                let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
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
                        let trigger = PendingTrigger::CreatureCardMilledWatch {
                            watcher_id,
                            watcher_card_id,
                            controller: watcher_controller,
                            milled_object: milled_obj,
                            milled_player,
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
            _ => {}
        }
    }

    // Drain any pending triggers (e.g., state-triggered abilities added during SBA)
    // into the appropriate APNAP bucket.
    for t in state.pending_triggers.drain(..) {
        if t.controller() == active_player {
            ap_triggers.push(t);
        } else {
            nap_triggers.push(t);
        }
    }

    let had_triggers = !ap_triggers.is_empty() || !nap_triggers.is_empty();

    // CR 603.3d: triggers go on the stack in APNAP order — AP first, then NAP.
    // For each trigger, if it has a target requirement, the player chooses the
    // target as it goes on the stack (handled by process_pending_trigger_pushes).
    state.pending_trigger_pushes_ap.extend(ap_triggers);
    state.pending_trigger_pushes_nap.extend(nap_triggers);

    // Mark all events as processed.
    state.trigger_event_index = events.len();

    // Process the queue: push triggers onto the stack one at a time, prompting
    // for targets where needed. Returns early if a prompt is set up.
    process_pending_trigger_pushes(state, registry);

    had_triggers
}

/// CR 603.3d: process the queue of triggers waiting to be pushed onto the
/// stack. For each trigger, if it has a target requirement, prompt the player
/// to choose targets (or auto-pick if there's exactly one legal target).
///
/// Returns when the queue is empty or when a prompt has been set up
/// (`state.awaiting_action.is_some()`). The caller (or `Action::ResolveChoice`
/// handler) will re-enter this function to continue processing the queue
/// after the player has made their choice.
pub fn process_pending_trigger_pushes(state: &mut GameState, registry: &CardRegistry) {
    while state.awaiting_action.is_none() {
        // Process AP queue first, then NAP.
        let trigger = if !state.pending_trigger_pushes_ap.is_empty() {
            state.pending_trigger_pushes_ap.remove(0)
        } else if !state.pending_trigger_pushes_nap.is_empty() {
            state.pending_trigger_pushes_nap.remove(0)
        } else {
            return;
        };

        // Look up the target requirement for this trigger.
        let card_id = trigger.behavior_card_id();
        let target_req = trigger.kind().and_then(|kind| {
            registry.get(card_id)
                .and_then(|b| b.card_data().triggered_abilities.into_iter()
                    .find(|t| t.kind == kind)
                    .and_then(|t| t.target_requirement))
        });

        let Some(req) = target_req else {
            // Untargeted: push directly onto the stack.
            state.stack.push(StackEntry::Trigger(trigger));
            continue;
        };

        // Compute valid targets via the same helper as spell casting.
        let source_id = trigger.source_object().unwrap_or(crate::ids::ObjectId(0));
        let controller = trigger.controller();
        let Some(behavior) = registry.get(card_id) else {
            // Card behavior not found — skip the trigger (shouldn't happen).
            continue;
        };
        let valid_targets = crate::engine::valid_targets_for_req(
            state, controller, source_id, &req, behavior, registry,
        );

        match valid_targets.len() {
            0 => {
                // CR 603.3c: a triggered ability with no legal targets is
                // removed from the stack (i.e., never goes on it).
                state.log(crate::state::LogLevel::Event,
                    format!("Trigger removed: no legal targets ({})", trigger.display_name(registry)));
            }
            1 => {
                // Auto-pick the single legal target.
                let target = valid_targets[0].clone();
                let mut t = trigger;
                t.set_chosen_targets(vec![target]);
                state.stack.push(StackEntry::Trigger(t));
            }
            _ => {
                // Multiple legal targets: prompt the player. Stash the trigger
                // back at the front of its queue, set up awaiting_action, and
                // return. When the player chooses, AttachTargetToPendingTrigger
                // will pop the trigger, attach the target, and re-enter here.
                let is_ap = controller == state.active_player;
                if is_ap {
                    state.pending_trigger_pushes_ap.insert(0, trigger.clone());
                } else {
                    state.pending_trigger_pushes_nap.insert(0, trigger.clone());
                }
                let description = format!(
                    "{}: choose target",
                    trigger.display_name_with_state(registry, Some(state))
                );
                state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: source_id,
                    choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                        description,
                        options: valid_targets,
                        optional: false,
                        effect: crate::state::PendingEffect::AttachTargetToPendingTrigger,
                    },
                });
                return;
            }
        }
    }
}

/// Resolve the top trigger from the stack.
/// Returns true if a trigger was resolved, false if the top of stack is not a trigger.
///
/// # Panics
/// Panics if the stack is mutated between the top-of-stack check and the pop
/// such that the popped entry is missing or not a trigger. Under normal
/// single-threaded use this cannot happen.
pub fn resolve_next_trigger(state: &mut GameState, registry: &CardRegistry) -> bool {
    // Check if the top of stack is a trigger.
    let is_trigger = state.stack.last()
        .is_some_and(|e| matches!(e, crate::state::StackEntry::Trigger(_)));
    if !is_trigger {
        return false;
    }
    let entry = state.stack.pop().expect("stack must have trigger entry");
    let crate::state::StackEntry::Trigger(trigger) = entry else { unreachable!() };

    // CR 608.2b: re-check target legality before resolving. If the trigger
    // has targets and ALL are now illegal, the ability is countered by game rules.
    let targets = trigger.chosen_targets();
    if !targets.is_empty() {
        let card_id = trigger.behavior_card_id();
        let controller = trigger.controller();
        let target_req = registry.get(card_id)
            .and_then(|behavior| {
                let data = behavior.card_data();
                let kind = trigger.kind();
                kind.and_then(|k| {
                    data.triggered_abilities.iter()
                        .find(|ta| ta.kind == k)
                        .and_then(|ta| ta.target_requirement.clone())
                })
            })
            .unwrap_or(crate::cards::TargetRequirement::None);
        let any_legal = targets.iter().any(|t| {
            crate::stack::is_target_legal(state, t, &target_req, controller, registry)
        });
        if !any_legal {
            let name = trigger.display_name(registry);
            state.log(crate::state::LogLevel::Event, format!("{name} fizzled (all targets illegal)"));
            return true;
        }
    }

    match trigger {
        PendingTrigger::EnteredBattlefield { object_id, card_id, chosen_targets, .. } => {
            // Per MTG rules, ETB triggers resolve even if the source has left
            // the battlefield. The trigger was already on the stack when the
            // creature entered. Individual handlers can check zone if needed.
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_enter_battlefield(state, object_id, &chosen_targets, registry);
            }
        }
        PendingTrigger::SelfDies { dead_id, dead_card_id, chosen_targets, .. } => {
            if let Some(behavior) = registry.get(dead_card_id) {
                behavior.on_dies(state, dead_id, &chosen_targets, registry);
            }
        }
        PendingTrigger::DeathWatch { watcher_id, watcher_card_id, dead_id, dead_controller, dead_damaged_by, dead_toughness, dead_is_token, chosen_targets, .. } => {
            // Per MTG rules, death triggers fire even if the watcher died
            // simultaneously (e.g., Falkenrath Noble + board wipe). The trigger
            // was created when the watcher was last known to be on the battlefield.
            if let Some(behavior) = registry.get(watcher_card_id) {
                behavior.on_any_creature_dies(state, watcher_id, dead_id, dead_controller, &dead_damaged_by, dead_toughness, dead_is_token, &chosen_targets, registry);
            }
        }
        PendingTrigger::EnterWatch { watcher_id, watcher_card_id, entered_id, entered_controller, .. } => {
            if let Some(behavior) = registry.get(watcher_card_id) {
                behavior.on_any_creature_enters(state, watcher_id, entered_id, entered_controller, registry);
            }
        }
        PendingTrigger::CombatDamageToPlayer { creature_id, creature_card_id, damaged_player, amount, .. } => {
            if let Some(behavior) = registry.get(creature_card_id) {
                behavior.on_combat_damage_to_player(state, creature_id, damaged_player, amount, registry);
            }
        }
        PendingTrigger::CombatDamageToCreature { creature_id, creature_card_id, damaged_creature, amount, .. } => {
            if state.get_object(creature_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(creature_card_id) {
                    behavior.on_deals_combat_damage_to_creature(state, creature_id, damaged_creature, amount, registry);
                }
            }
        }
        PendingTrigger::CombatDamageWatch { watcher_id, watcher_card_id, source_id, damaged_player, amount, .. } => {
            if state.get_object(watcher_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_combat_damage_to_player(state, watcher_id, source_id, damaged_player, amount, registry);
                }
            }
        }
        PendingTrigger::DamageToPlayerWatch { watcher_id, watcher_card_id, source_id, damaged_player, amount, .. } => {
            if state.get_object(watcher_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_damage_to_player(state, watcher_id, source_id, damaged_player, amount, registry);
                }
            }
        }
        PendingTrigger::EndCombatTrigger { object_id, card_id, .. } => {
            if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_end_combat(state, object_id, registry);
                }
            }
        }
        PendingTrigger::UpkeepTrigger { object_id, card_id, chosen_targets, .. } => {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_upkeep(state, object_id, &chosen_targets, registry);
            }
        }
        PendingTrigger::EndStepTrigger { object_id, card_id, chosen_targets, .. } => {
            if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_end_step(state, object_id, &chosen_targets, registry);
                }
            }
        }
        PendingTrigger::SpellCastWatch { watcher_id, watcher_card_id, caster, spell_id, chosen_targets, .. } => {
            if state.get_object(watcher_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_spell_cast(state, watcher_id, caster, spell_id, &chosen_targets, registry);
                }
            }
        }
        PendingTrigger::LeftBattlefield { object_id, card_id, .. } => {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_leave_battlefield(state, object_id, registry);
            }
        }
        PendingTrigger::AttacksTrigger { object_id, card_id, chosen_targets, .. } => {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_attacks(state, object_id, &chosen_targets, registry);
            }
        }
        PendingTrigger::BlocksTrigger { object_id, card_id, blocked_attacker, .. } => {
            if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_blocks(state, object_id, blocked_attacker, registry);
                }
            }
        }
        PendingTrigger::AttackWatch { watcher_id, watcher_card_id, attacker_id, attacker_controller, .. } => {
            if state.get_object(watcher_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(watcher_card_id) {
                    behavior.on_any_creature_attacks(state, watcher_id, attacker_id, attacker_controller, registry);
                }
            }
        }
        PendingTrigger::BecomesBlockedTrigger { object_id, card_id, blocker_id, .. } => {
            if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_becomes_blocked(state, object_id, blocker_id, registry);
                }
            }
        }
        PendingTrigger::StateTriggered { object_id, card_id, .. } => {
            // Clear the "on stack" flag so the trigger can fire again if the
            // condition is still true after resolution.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.state_trigger_on_stack = false;
            }
            // Only resolve if the object is still on the battlefield.
            if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_state_trigger(state, object_id, registry);
                }
            }
        }
        PendingTrigger::CreatureCardMilledWatch { watcher_id, watcher_card_id, milled_object, milled_player, .. } => {
            if let Some(behavior) = registry.get(watcher_card_id) {
                behavior.on_creature_card_milled(state, watcher_id, milled_object, milled_player, registry);
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

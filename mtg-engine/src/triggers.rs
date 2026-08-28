use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameState, StackEntry};
use crate::types::Zone;

/// A triggered ability that has been collected but not yet resolved.
/// These are placed on `pending_triggers` in APNAP order, then resolved
/// LIFO (non-active player's triggers resolve first).
///
/// Every trigger is a *source* — the object whose ability this is — plus the
/// *event* it triggered on. Before this it was one enum of twenty variants,
/// each spelling the source out again under whichever name that variant's
/// author picked (`watcher_id`, `object_id`, `dead_id`, `creature_id`), which
/// is why `controller`, `source_object`, `behavior_card_id`, `chosen_targets`
/// and `display_name` were all twenty-arm matches over the same four fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingTrigger {
    pub source: TriggerSource,
    pub event: TriggerEvent,
}

/// The object whose triggered ability this is.
///
/// Captured when the trigger is created, which makes it last known information
/// (CR 603.10a): the permanent may be gone by the time the trigger resolves,
/// and the ability resolves anyway (CR 113.7a).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriggerSource {
    pub id: ObjectId,
    pub card_id: CardId,
    /// Who controls the ability. For a leave-the-battlefield trigger this is
    /// who controlled the permanent immediately before it left (CR 603.10c).
    pub controller: PlayerId,
    /// The `TriggeredAbilityDef` description, for the stack view.
    pub description: String,
    /// CR 603.3d: targets chosen as the trigger goes on the stack. Empty for
    /// untargeted triggers.
    #[serde(default)]
    pub chosen_targets: Vec<Target>,
    /// Whether the permanent was on its back face when this triggered.
    ///
    /// A double-faced card's two faces have different abilities, and the
    /// ability that triggered belongs to one of them. It keeps belonging to
    /// that face after the permanent flips — so an intervening-if re-checked at
    /// resolution (CR 603.4) has to test the condition of the face that
    /// triggered, not whatever face is up by then.
    #[serde(default)]
    pub from_back_face: bool,
}

impl TriggerSource {
    #[must_use]
    pub fn new(id: ObjectId, card_id: CardId, controller: PlayerId, description: impl Into<String>) -> Self {
        Self { id, card_id, controller, description: description.into(),
               chosen_targets: Vec::new(), from_back_face: false }
    }

    /// Record which face this triggered from (CR 712.8 — the faces have
    /// different abilities).
    #[must_use]
    pub fn from_face(mut self, is_transformed: bool) -> Self {
        self.from_back_face = is_transformed;
        self
    }
}

/// Last known information about a creature that died, captured before the zone
/// change cleared its battlefield state (CR 603.10a).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeadCreature {
    pub id: ObjectId,
    pub controller: PlayerId,
    pub damaged_by: Vec<ObjectId>,
    pub toughness: i32,
    pub is_token: bool,
}

/// What the trigger triggered on.
///
/// One variant per `CardBehavior` hook the trigger dispatches to, carrying
/// only what that hook needs beyond the source. "Self" and "watcher" variants
/// stay distinct because they call different hooks and match different
/// `TriggerKind`s — a card can have both, and Blood Artist's "whenever this or
/// another creature dies" must fire its watcher ability when it dies itself.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TriggerEvent {
    /// This creature died.
    SelfDies,
    /// A creature died, watched by another permanent.
    CreatureDied { dead: DeadCreature },
    /// This permanent entered the battlefield.
    SelfEntered,
    /// A creature entered the battlefield, watched by another permanent.
    CreatureEntered { entered: ObjectId, entered_controller: PlayerId },
    /// This creature dealt combat damage to a player.
    CombatDamageToPlayer { damaged_player: PlayerId, amount: u32 },
    /// A creature dealt combat damage to a player, watched.
    AnyCombatDamageToPlayer { dealer: ObjectId, damaged_player: PlayerId, amount: u32 },
    /// Any damage, combat or not, was dealt to a player, watched.
    AnyDamageToPlayer { dealer: ObjectId, damaged_player: PlayerId, amount: u32 },
    /// This creature dealt combat damage to another creature.
    CombatDamageToCreature { damaged_creature: ObjectId, amount: u32 },
    /// A spell was cast, watched.
    SpellCast { caster: PlayerId, spell_id: ObjectId },
    /// A creature attacked, and this trigger belongs to it or to something
    /// attached to it. `attacker` and `defending_player` are snapshotted at
    /// declaration (CR 508.1) — by resolution the creature may be dead and an
    /// Equipment may be on something else.
    Attacks { attacker: ObjectId, defending_player: PlayerId },
    /// A creature attacked, watched by any permanent (including the attacker
    /// itself — Instigator Gang must see its own attack).
    CreatureAttacked { attacker: ObjectId, attacker_controller: PlayerId },
    /// This creature blocked an attacker.
    Blocks { blocked_attacker: ObjectId },
    /// This attacking creature became blocked.
    BecomesBlocked { blocker_id: ObjectId },
    /// A creature card was milled from an opponent's library, watched.
    CreatureCardMilled { milled_object: ObjectId, milled_player: PlayerId },
    /// This permanent left the battlefield.
    LeftBattlefield,
    /// An upkeep step began.
    Upkeep,
    /// An end step began.
    EndStep,
    /// The end of combat step began.
    EndCombat,
    /// A state-triggered ability (CR 603.8), raised during SBA processing. It
    /// will not fire again while it is already on the stack.
    StateTriggered,
    /// A delayed triggered ability that exiles a specific token at end of
    /// combat (CR 603.7) — Geist of Saint Traft's Angel. Fires even if the
    /// source permanent has since left the battlefield (CR 603.7d).
    DelayedTokenExile { target_id: ObjectId },
}

impl TriggerEvent {
    /// The `TriggerKind` whose `TriggeredAbilityDef` describes this trigger.
    ///
    /// `None` for the two triggers the engine raises itself rather than
    /// reading off a card: state-triggered abilities and delayed exile.
    #[must_use]
    pub fn kind(&self) -> Option<crate::cards::TriggerKind> {
        use crate::cards::TriggerKind as K;
        Some(match self {
            TriggerEvent::SelfDies => K::SelfDies,
            TriggerEvent::CreatureDied { .. } => K::AnyCreatureDies,
            TriggerEvent::SelfEntered => K::EntersBattlefield,
            TriggerEvent::CreatureEntered { .. } => K::AnyCreatureEnters,
            TriggerEvent::CombatDamageToPlayer { .. } => K::CombatDamageToPlayer,
            TriggerEvent::AnyCombatDamageToPlayer { .. } => K::AnyCombatDamageToPlayer,
            TriggerEvent::AnyDamageToPlayer { .. } => K::AnyDamageToPlayer,
            TriggerEvent::CombatDamageToCreature { .. } => K::DealsCombatDamageToCreature,
            TriggerEvent::SpellCast { .. } => K::SpellCast,
            TriggerEvent::Attacks { .. } => K::Attacks,
            TriggerEvent::CreatureAttacked { .. } => K::AnyCreatureAttacks,
            TriggerEvent::Blocks { .. } => K::Blocks,
            TriggerEvent::BecomesBlocked { .. } => K::BecomesBlocked,
            TriggerEvent::CreatureCardMilled { .. } => K::CreatureCardMilled,
            TriggerEvent::LeftBattlefield => K::LeavesBattlefield,
            TriggerEvent::Upkeep => K::Upkeep,
            TriggerEvent::EndStep => K::EndStep,
            TriggerEvent::EndCombat => K::EndCombat,
            TriggerEvent::StateTriggered | TriggerEvent::DelayedTokenExile { .. } => return None,
        })
    }

    /// How the stack view names a trigger of this kind.
    fn phrase(&self) -> &'static str {
        match self {
            TriggerEvent::SelfDies => "dies trigger",
            TriggerEvent::CreatureDied { .. }
            | TriggerEvent::CreatureEntered { .. }
            | TriggerEvent::AnyCombatDamageToPlayer { .. }
            | TriggerEvent::AnyDamageToPlayer { .. }
            | TriggerEvent::SpellCast { .. } => "triggered ability",
            TriggerEvent::SelfEntered => "ETB trigger",
            TriggerEvent::CombatDamageToPlayer { .. }
            | TriggerEvent::CombatDamageToCreature { .. } => "combat damage trigger",
            TriggerEvent::Attacks { .. } | TriggerEvent::CreatureAttacked { .. } => "attack trigger",
            TriggerEvent::Blocks { .. } | TriggerEvent::BecomesBlocked { .. } => "block trigger",
            TriggerEvent::CreatureCardMilled { .. } => "mill-watcher trigger",
            TriggerEvent::LeftBattlefield => "LTB trigger",
            TriggerEvent::Upkeep => "upkeep trigger",
            TriggerEvent::EndStep => "end step trigger",
            TriggerEvent::EndCombat => "end of combat trigger",
            TriggerEvent::StateTriggered => "state-triggered ability",
            TriggerEvent::DelayedTokenExile { .. } => "delayed exile trigger",
        }
    }
}

impl PendingTrigger {
    #[must_use]
    pub fn new(source: TriggerSource, event: TriggerEvent) -> Self {
        Self { source, event }
    }

    /// The player who controls this trigger.
    #[must_use]
    pub fn controller(&self) -> PlayerId {
        self.source.controller
    }

    /// The source object the trigger fires from — the "source" for target
    /// validation and the object whose `TriggeredAbilityDef` is consulted.
    #[must_use]
    pub fn source_object(&self) -> ObjectId {
        self.source.id
    }

    /// The card id of the behavior that owns this trigger.
    #[must_use]
    pub fn behavior_card_id(&self) -> CardId {
        self.source.card_id
    }

    /// The trigger kind, used to look up the matching `TriggeredAbilityDef`.
    #[must_use]
    pub fn kind(&self) -> Option<crate::cards::TriggerKind> {
        self.event.kind()
    }

    #[must_use]
    pub fn chosen_targets(&self) -> &[Target] {
        &self.source.chosen_targets
    }

    /// Display name for the stack view, including what the trigger does.
    #[must_use]
    pub fn display_name(&self, registry: &crate::cards::CardRegistry) -> String {
        self.display_name_with_state(registry, None)
    }

    /// Display name with optional game state, so a transformed permanent shows
    /// its back-face name. Every trigger kind gets this, not just upkeep —
    /// the source object is now on the trigger regardless of event.
    #[must_use]
    pub fn display_name_with_state(&self, registry: &crate::cards::CardRegistry, state: Option<&crate::state::GameState>) -> String {
        let is_transformed = state
            .and_then(|s| s.get_object(self.source.id))
            .is_some_and(|o| o.is_transformed);
        let name = registry
            .get(self.source.card_id)
            .and_then(|b| if is_transformed { b.back_face_data().map(|d| d.name) } else { None })
            .or_else(|| registry.card_data(self.source.card_id).map(|d| d.name))
            .unwrap_or_else(|| "Unknown".into());
        let phrase = self.event.phrase();
        if self.source.description.is_empty() {
            format!("{name}'s {phrase}")
        } else {
            format!("{name}'s {phrase} ({})", self.source.description)
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
pub mod collect;

pub fn collect_triggers(state: &mut GameState, registry: &CardRegistry) -> bool {
    let events = state.events.clone();
    let start = state.trigger_event_index;
    let active_player = state.active_player;

    let mut c = collect::Collector::new(active_player);

    for event in events.iter().skip(start) {
        collect::for_event(state, &events, event, registry, &mut c);
    }

    // Drain any pending triggers (e.g., state-triggered abilities added during SBA)
    // into the appropriate APNAP bucket.
    let pending: Vec<PendingTrigger> = state.pending_triggers.drain(..).collect();
    for t in pending {
        c.push(t);
    }

    let had_triggers = !c.is_empty();

    // CR 603.3d: triggers go on the stack in APNAP order — AP first, then NAP.
    // For each trigger, if it has a target requirement, the player chooses the
    // target as it goes on the stack (handled by process_pending_trigger_pushes).
    state.pending_trigger_pushes_ap.extend(c.ap);
    state.pending_trigger_pushes_nap.extend(c.nap);

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
        let card_id = trigger.source.card_id;
        let target_req = target_requirement(registry, card_id, trigger.event.kind());

        let Some(req) = target_req else {
            // Untargeted: push directly onto the stack.
            state.stack.push(StackEntry::Trigger(trigger));
            continue;
        };

        // Compute valid targets via the same helper as spell casting.
        let source_id = trigger.source.id;
        let controller = trigger.source.controller;
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
                t.source.chosen_targets = vec![target];
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

/// The `TargetRequirement` a card's `TriggeredAbilityDef` of this kind
/// declares, if any. Used both when the trigger goes on the stack (CR 603.3d)
/// and when its targets are re-checked on resolution (CR 608.2b).
fn target_requirement(
    registry: &CardRegistry,
    card_id: CardId,
    kind: Option<crate::cards::TriggerKind>,
) -> Option<crate::cards::TargetRequirement> {
    let kind = kind?;
    registry.get(card_id)?
        .card_data()
        .triggered_abilities
        .into_iter()
        .find(|t| t.kind == kind)
        .and_then(|t| t.target_requirement)
}

/// CR 603.7d: the delayed ability exiles the token it was created for, if that
/// token is still on the battlefield.
fn exile_delayed_token(state: &mut GameState, target_id: ObjectId, registry: &CardRegistry) {
    if state.get_object(target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
        state.move_object(target_id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            "Token exiled by delayed end-of-combat trigger".into());
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
    if !trigger.source.chosen_targets.is_empty() {
        let card_id = trigger.source.card_id;
        let controller = trigger.source.controller;
        let target_req = target_requirement(registry, card_id, trigger.event.kind())
            .unwrap_or(crate::cards::TargetRequirement::None);
        // Both halves of legality, the way `resolve_spell` already does it:
        // the generic zone/hexproof/filter check AND the card's own
        // restriction. Checking only the generic half let a trigger resolve
        // against a target that had stopped satisfying the card's wording —
        // Grimgrin's "creature the defending player controls" survived the
        // creature changing controller in response.
        let any_legal = trigger.source.chosen_targets.iter().any(|t| {
            crate::stack::is_target_legal(state, t, &target_req, controller, Some(trigger.source.id), registry)
                && registry.get(card_id)
                    .is_some_and(|b| b.is_valid_target(state, controller, t, registry))
        });
        if !any_legal {
            let name = trigger.display_name(registry);
            state.log(crate::state::LogLevel::Event, format!("{name} fizzled (all targets illegal)"));
            return true;
        }
    }

    // CR 113.7a: a triggered ability on the stack exists independently of its
    // source, so removing the source after the trigger is on the stack does
    // not counter it. Half these arms used to gate on the source still being
    // on the battlefield and half did not — the two halves of the same pair
    // (a creature's own combat-damage trigger and a watcher's) disagreed —
    // which silently cancelled Reaper from the Abyss's end-step destruction
    // and cost Rakish Heir its counter when the Heir traded in combat. There
    // is one rule and it is stated here: the source's zone is not consulted.
    // A handler that genuinely needs its permanent present checks for itself.
    let PendingTrigger { source, event } = trigger;
    let targets = source.chosen_targets;
    let Some(behavior) = registry.get(source.card_id) else {
        // The delayed exile trigger is raised by the engine and needs no
        // behavior; everything else is a card ability and cannot resolve
        // without one.
        if let TriggerEvent::DelayedTokenExile { target_id } = event {
            exile_delayed_token(state, target_id, registry);
        }
        return true;
    };
    let self_id = source.id;
    // CR 603.4: the ability's own condition is re-checked on resolution, and
    // "its own" means the face it triggered from — which is not necessarily the
    // face that is up now.
    state.resolving_trigger_from_back_face = Some(source.from_back_face);
    match event {
        TriggerEvent::SelfEntered => {
            behavior.on_enter_battlefield(state, self_id, &targets, registry);
        }
        TriggerEvent::SelfDies => {
            behavior.on_dies(state, self_id, &targets, registry);
        }
        TriggerEvent::CreatureDied { dead } => {
            behavior.on_any_creature_dies(state, self_id, dead.id, dead.controller,
                &dead.damaged_by, dead.toughness, dead.is_token, &targets, registry);
        }
        TriggerEvent::CreatureEntered { entered, entered_controller } => {
            behavior.on_any_creature_enters(state, self_id, entered, entered_controller, registry);
        }
        TriggerEvent::CombatDamageToPlayer { damaged_player, amount } => {
            behavior.on_combat_damage_to_player(state, self_id, damaged_player, amount, registry);
        }
        TriggerEvent::CombatDamageToCreature { damaged_creature, amount } => {
            behavior.on_deals_combat_damage_to_creature(state, self_id, damaged_creature, amount, registry);
        }
        TriggerEvent::AnyCombatDamageToPlayer { dealer, damaged_player, amount } => {
            behavior.on_any_combat_damage_to_player(state, self_id, dealer, damaged_player, amount, registry);
        }
        TriggerEvent::AnyDamageToPlayer { dealer, damaged_player, amount } => {
            behavior.on_any_damage_to_player(state, self_id, dealer, damaged_player, amount, registry);
        }
        TriggerEvent::EndCombat => behavior.on_end_combat(state, self_id, registry),
        TriggerEvent::Upkeep => behavior.on_upkeep(state, self_id, &targets, registry),
        TriggerEvent::EndStep => behavior.on_end_step(state, self_id, &targets, registry),
        TriggerEvent::SpellCast { caster, spell_id } => {
            behavior.on_spell_cast(state, self_id, caster, spell_id, &targets, registry);
        }
        TriggerEvent::LeftBattlefield => behavior.on_leave_battlefield(state, self_id, registry),
        TriggerEvent::Attacks { attacker, defending_player } => {
            let attack = crate::cards::AttackInfo::new(attacker, defending_player);
            behavior.on_attacks(state, self_id, attack, &targets, registry);
        }
        TriggerEvent::CreatureAttacked { attacker, attacker_controller } => {
            behavior.on_any_creature_attacks(state, self_id, attacker, attacker_controller, registry);
        }
        TriggerEvent::Blocks { blocked_attacker } => {
            behavior.on_blocks(state, self_id, blocked_attacker, registry);
        }
        TriggerEvent::BecomesBlocked { blocker_id } => {
            behavior.on_becomes_blocked(state, self_id, blocker_id, registry);
        }
        TriggerEvent::CreatureCardMilled { milled_object, milled_player } => {
            behavior.on_creature_card_milled(state, self_id, milled_object, milled_player, registry);
        }
        TriggerEvent::StateTriggered => {
            // Clear the "on stack" flag so the ability can fire again if the
            // condition is still true after resolution (CR 603.8).
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.state_trigger_on_stack = false;
            }
            behavior.on_state_trigger(state, self_id, registry);
        }
        TriggerEvent::DelayedTokenExile { target_id } => {
            exile_delayed_token(state, target_id, registry);
        }
    }


    state.resolving_trigger_from_back_face = None;
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

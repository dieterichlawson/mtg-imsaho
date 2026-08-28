//! Turning game events into pending triggered abilities.
//!
//! One function per [`GameEvent`] that can trigger something. They were arms
//! of a single 722-line `collect_triggers`.

use crate::ids::PlayerId;
use crate::triggers::PendingTrigger;

mod combat;
mod damage;
mod timing;
mod zones;

/// Accumulates triggers in APNAP order (CR 603.3b): the active player's
/// triggers go on the stack first, then each other player's.
///
/// Within one player's group the order is the order the watchers were
/// scanned, and every scan goes through `objects_in_id_order` so that order is
/// object id — fixed, and the same on a replay. Scanning `state.objects`
/// directly put simultaneous triggers on the stack in HashMap order, which is
/// seeded per process.
///
/// CR 603.3b actually lets a player with several simultaneous triggers choose
/// the order to put them on the stack, and this does not ask. Object id is a
/// deterministic stand-in for that choice, not the choice itself.
///
/// Every arm used to spell this out as `if controller == active_player {
/// ap.push(t) } else { nap.push(t) }` — twenty-one times.
pub(crate) struct Collector {
    pub active_player: PlayerId,
    pub ap: Vec<PendingTrigger>,
    pub nap: Vec<PendingTrigger>,
}

impl Collector {
    pub fn new(active_player: PlayerId) -> Self {
        Self { active_player, ap: Vec::new(), nap: Vec::new() }
    }

    /// Add a trigger, filing it by whether its controller is the active
    /// player. The controller is read off the trigger's source, so the twenty
    /// `if controller == active_player { ap.push } else { nap.push }` blocks
    /// that used to follow every construction are gone.
    pub fn push(&mut self, trigger: PendingTrigger) {
        if trigger.source.controller == self.active_player {
            self.ap.push(trigger);
        } else {
            self.nap.push(trigger);
        }
    }

    /// Build a trigger from its source parts and file it. `description` is the
    /// source's `TriggeredAbilityDef` text, shown on the stack.
    pub fn emit(
        &mut self,
        id: crate::ids::ObjectId,
        card_id: crate::ids::CardId,
        controller: PlayerId,
        description: String,
        event: crate::triggers::TriggerEvent,
    ) {
        self.push(PendingTrigger::new(
            crate::triggers::TriggerSource::new(id, card_id, controller, description),
            event,
        ));
    }

    /// Emit a trigger, recording which face of a double-faced card it fired
    /// from (CR 712.8 — the two faces have different abilities, and an
    /// intervening-if re-checked on resolution belongs to the face that
    /// triggered).
    pub fn emit_from_face(
        &mut self,
        id: crate::ids::ObjectId,
        card_id: crate::ids::CardId,
        controller: PlayerId,
        description: String,
        event: crate::triggers::TriggerEvent,
        is_transformed: bool,
    ) {
        self.push(PendingTrigger::new(
            crate::triggers::TriggerSource::new(id, card_id, controller, description)
                .from_face(is_transformed),
            event,
        ));
    }

    pub fn is_empty(&self) -> bool {
        self.ap.is_empty() && self.nap.is_empty()
    }
}

/// Was this permanent on the battlefield when the event being collected
/// happened?
///
/// Not the same question as "is it on the battlefield now". An ability
/// triggers when its event occurs (CR 603.2), and the game performs state-based
/// actions — noticing the deaths that event caused — before it puts any
/// triggered abilities on the stack (CR 704.3). So by collection time the
/// creature that dealt combat damage, and the watcher that saw it, may both
/// already be in a graveyard, and both abilities triggered all the same
/// (CR 113.7a).
///
/// A permanent counts as present if it is still there, or if it left the
/// battlefield somewhere in this same batch of events. Ordering within the
/// batch is not consulted: a batch is one resolution or one turn-based action,
/// so a departure in it is a consequence of the event rather than something
/// that preceded it.
pub(crate) fn was_on_the_battlefield(
    state: &crate::state::GameState,
    events: &[crate::events::GameEvent],
    id: crate::ids::ObjectId,
) -> bool {
    use crate::events::GameEvent as E;
    if state.get_object(id).is_some_and(|o| o.zone == crate::types::Zone::Battlefield) {
        return true;
    }
    events.iter().any(|e| match e {
        E::LeftBattlefield { object, .. } | E::CreatureDied { object, .. } => *object == id,
        _ => false,
    })
}

/// Dispatch one event to whichever collector cares about it.
pub(crate) fn for_event(
    state: &mut crate::state::GameState,
    events: &[crate::events::GameEvent],
    event: &crate::events::GameEvent,
    registry: &crate::cards::CardRegistry,
    c: &mut Collector,
) {
    use crate::events::GameEvent as E;
    match event {
        E::EnteredBattlefield { .. } => zones::entered_battlefield(state, events, event, registry, c),
        E::CreatureDied { .. } => zones::creature_died(state, events, event, registry, c),
        E::LeftBattlefield { .. } => zones::left_battlefield(state, events, event, registry, c),
        E::CombatDamageDealt { .. } => damage::combat_damage(state, events, event, registry, c),
        E::NonCombatDamageDealt { .. } => damage::noncombat_damage(state, events, event, registry, c),
        E::StepStarted { .. } => timing::step_started(state, events, event, registry, c),
        E::SpellCast { .. } => timing::spell_cast(state, events, event, registry, c),
        E::AttackersDeclared { .. } => combat::attackers_declared(state, events, event, registry, c),
        E::BlockersDeclared { .. } => combat::blockers_declared(state, events, event, registry, c),
        E::CreatureCardMilled { .. } => zones::creature_card_milled(state, events, event, registry, c),
        _ => {}
    }
}

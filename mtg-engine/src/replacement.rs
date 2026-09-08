//! Replacement effects (CR 614): watching for an event and changing what
//! happens instead.
//!
//! One mechanism, one place it is applied. Before this there were seven —
//! `replacement_effects` (a closed engine enum), `enters_tapped`,
//! `entering_with_counters`, `modify_creature_entering_counters`,
//! `entering_modifier_zones`, `enters_as_copy` and
//! `replace_combat_damage_to_player` — each consulted from exactly one site,
//! so a new call site had to remember all seven and none of them could express
//! CR 614.5 (an effect applies at most once per event) or CR 616.1 (the
//! affected player orders several applicable ones).
//!
//! The shape follows XMage's `replaceEvent`: the engine builds the event it is
//! about to perform, offers it to everything that might replace it, and
//! performs whatever comes back.

use crate::cards::CardRegistry;
use crate::events::DamageTarget;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{CounterType, Zone};

/// A permanent on its way onto the battlefield.
///
/// Replacement effects here change *how* it arrives, not whether — CR 614.1c
/// ("enters with counters") and 614.1d ("enters tapped", "enters as a copy")
/// are all modifications of this one event.
#[derive(Debug, Clone, PartialEq)]
pub struct EnteringPermanent {
    pub object: ObjectId,
    /// Where it is coming from. `None` for a token, which comes from nowhere.
    pub from: Option<Zone>,
    pub controller: PlayerId,
    pub tapped: bool,
    pub counters: Vec<(CounterType, u32)>,
    /// The permanent it enters as a copy of (CR 706.9), if any.
    ///
    /// A permanent rather than a card: CR 706.2 copies the *copiable values*
    /// of the chosen object, which for an object that is itself a copy are
    /// the copied ones, and for a token are the token's own. Naming the card
    /// lost both.
    pub copy_of: Option<ObjectId>,
}

/// An event a replacement effect may act on before it happens.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceableEvent {
    EntersBattlefield(EnteringPermanent),
    /// One or more tokens are about to be created under `controller`.
    CreatesTokens { controller: PlayerId, count: u32 },
    /// `player` would draw a card with an empty library.
    DrawsFromEmptyLibrary { player: PlayerId },
    /// Damage is about to be dealt.
    DealsDamage {
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        combat: bool,
    },
}

/// What a replacement effect did to an event.
pub enum Replacement {
    /// The event happens, but like this instead.
    Modified(ReplaceableEvent),
    /// The event does not happen at all. The effect has already done whatever
    /// it does instead — Laboratory Maniac has won the game, Undead Alchemist
    /// has milled.
    Replaced,
}

/// Offer `event` to everything that might replace it and return the event as
/// it should actually happen, or `None` if it was replaced entirely.
///
/// CR 614.5: a given replacement effect applies at most once to a given
/// event, which is why each candidate is asked once and dropped afterwards.
///
/// CR 616.1 says the affected player chooses the order when several apply.
/// There is no prompt: candidates are taken in a deterministic order (by
/// object id).
///
/// This used to say that no board in this pool could produce two effects
/// applicable to one event, which is FALSE and was the reason nobody looked
/// (issue #323). Undead Alchemist ("if a Zombie you control would deal combat
/// damage to a player, instead that player mills that many") plus Inquisitor's
/// Flail ("deals double that damage instead") both modify one combat damage
/// event and were reached in ordinary play; so were Undead Alchemist plus
/// Ghostly Possession's prevention. With the Flail the fixed order is a wrong
/// result, not just a missing choice — the defending player would apply the
/// Alchemist first and mill 2, and the engine doubles first and mills 4.
///
/// Two things have to happen together to fix it, which is why neither is
/// here yet. This loop is one place a prompt would go, but it cannot see the
/// other candidates: `damage.rs::deal_damage_to_player` applies combat-damage
/// prevention and the Flail's multiplier upstream, hardcoded, before this is
/// ever called. They have to become candidates here first. And a prompt in
/// this loop suspends the middle of a damage event, which nothing in the
/// damage pipeline can resume today — CR 510.2 deals combat damage
/// simultaneously, so the resumption point is per source-and-target, not per
/// step.
pub fn apply(
    state: &mut GameState,
    event: ReplaceableEvent,
    registry: &CardRegistry,
) -> Option<ReplaceableEvent> {
    let mut candidates: Vec<(ObjectId, CardId)> = state
        .objects
        .values()
        .filter(|o| {
            registry
                .get(o.card_id)
                .is_some_and(|b| b.replacement_zones().contains(&o.zone))
        })
        .map(|o| (o.id, o.card_id))
        .collect();
    candidates.sort_by_key(|(id, _)| id.0);

    let mut current = event;
    for (object, card_id) in candidates {
        let Some(behavior) = registry.get(card_id) else { continue };
        match behavior.replace_event(state, object, &current, registry) {
            None => {}
            Some(Replacement::Replaced) => return None,
            Some(Replacement::Modified(next)) => current = next,
        }
    }
    Some(current)
}

/// Run `event` through the replacement layer and return the entering
/// permanent as it should arrive.
///
/// Entering the battlefield can be modified but never prevented, so this
/// always yields an `EnteringPermanent`.
pub fn for_entering(
    state: &mut GameState,
    entering: EnteringPermanent,
    registry: &CardRegistry,
) -> EnteringPermanent {
    // CR 616.1, and the ruling Essence of the Wild is written against:
    // "Replacement effects that modify how a creature enters are applied in
    // the following order: first control-changing effects, then copy effects,
    // then all other effects."
    //
    // The order matters because a copy effect decides *what is entering*, and
    // the rest of the effects belong to whatever that turns out to be:
    // "Other 'enters' replacement abilities printed on the creature entering
    // won't be applied because the creature will already be Essence of the
    // Wild at that point (and therefore it won't have those abilities). For
    // example, a creature that normally enters tapped will enter as an
    // untapped Essence of the Wild."
    //
    // So: one pass that keeps only the copy decision, then a second pass over
    // the same candidates for everything else — by which time the entering
    // permanent's own abilities are read from the card it is copying.
    let fallback = entering.clone();
    let after_copy = run_entering_pass(state, entering, registry, Pass::CopyOnly)
        .unwrap_or_else(|| fallback.clone());
    run_entering_pass(state, after_copy.clone(), registry, Pass::EverythingElse)
        .unwrap_or(after_copy)
}

/// Ask for the enters-as-a-copy choices that deferred entries are waiting on
/// (CR 614.12b), one at a time.
///
/// `move_object` queues an object here rather than putting it onto the
/// battlefield with the choice unanswered. The engine calls this before any
/// player receives priority; each answer records itself through
/// `record_entry_choice`, which completes that object's entry and comes back
/// here for the next one. A queue rather than a single prompt because one
/// effect can bring several such cards in at once (Grimoire of the Dead
/// returns every creature card in every graveyard).
pub fn process_pending_entry_choices(state: &mut GameState, registry: &CardRegistry) {
    while state.awaiting_action.is_none() {
        let Some(&object) = state.pending_entry_choices.first() else { return };
        let Some(controller) = state.get_object(object).map(|o| o.controller) else {
            state.pending_entry_choices.remove(0);
            continue;
        };
        // "A copy of any creature on the battlefield" is a choice, not a
        // target (CR 115.1, 614.12b), so hexproof and protection do not
        // narrow it.
        let options: Vec<crate::actions::Target> = state
            .all_objects_in_zone(Zone::Battlefield)
            .iter()
            .filter(|o| state.is_creature(o.id, registry))
            .map(|o| crate::actions::Target::Object(o.id))
            .collect();
        if options.is_empty() {
            // Nothing to copy: the choice cannot be made, so the permanent
            // enters as its printed self.
            record_entry_choice(state, object, crate::state::EnterAsCopyChoice::Declined, registry);
            continue;
        }
        let name = state.obj_name(object);
        crate::cards::helpers::present_optional_target_choice(
            state,
            object,
            controller,
            options,
            crate::state::PendingEffect::EnterAsCopy { object },
            &format!("{name}: you may have it enter as a copy of a creature on the battlefield"),
            registry,
        );
    }
}

/// Drop from a pending enters-as-a-copy prompt any creature that has since
/// left the battlefield, and decline the choice outright if none is left.
///
/// The choice is offered before state-based actions run (CR 614.12b: the
/// entry is waiting on it, so SBAs have nothing to say about the permanent
/// yet), and the answer is given at the decision point after them. A
/// creature that was on the battlefield when the list was built can be dead
/// by the time the player reads it — a Grimoire of the Dead returning
/// thirteen creatures at once kills some of them to the legend rule in the
/// same breath — and a copy of a creature that is no longer there is not a
/// choice the game can offer (CR 608.2d).
pub fn refresh_pending_entry_choice(state: &mut GameState, registry: &CardRegistry) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseTarget {
        options, effect: crate::state::PendingEffect::EnterAsCopy { object }, .. }, .. })
        = &state.awaiting_action else { return };
    let object = *object;
    let live: Vec<crate::actions::Target> = options.iter().filter(|t| match t {
        crate::actions::Target::Object(id) => state.get_object(*id)
            .is_some_and(|o| o.zone == Zone::Battlefield) && state.is_creature(*id, registry),
        _ => false,
    }).cloned().collect();
    if live.len() == options.len() {
        return;
    }
    if live.is_empty() {
        // Nothing left to copy, so the permanent enters as its printed self,
        // exactly as it would have had the board been empty when asked.
        state.awaiting_action = None;
        record_entry_choice(state, object, crate::state::EnterAsCopyChoice::Declined, registry);
        return;
    }
    if let Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseTarget {
        options, .. }, .. }) = &mut state.awaiting_action {
        *options = live;
    }
}

/// Record the answer to an enters-as-a-copy choice and finish that object's
/// entry.
///
/// The entry was deferred (see `GameState::move_object`), so this is where it
/// actually happens — with the answer already on the object, the card's own
/// replacement effect can turn it into `copy_of` as it enters.
pub fn record_entry_choice(
    state: &mut GameState,
    object: ObjectId,
    choice: crate::state::EnterAsCopyChoice,
    registry: &CardRegistry,
) {
    if let Some(obj) = state.get_object_mut(object) {
        obj.entering_copy_choice = choice;
    }
    state.pending_entry_choices.retain(|&id| id != object);
    // The entry the deferral belonged to may have asked for a controller
    // other than the owner (Grimoire of the Dead, Moldgraf Monstrosity,
    // Fiend Hunter). It was held rather than written, so it is applied now,
    // as part of the entry it came with (issues #335-#349).
    match state.pending_entry_controllers.remove(&object) {
        Some(controller) =>
            state.move_object_under_control(object, Zone::Battlefield, controller, registry),
        None => state.move_object(object, Zone::Battlefield, registry),
    };
}

/// Which half of the entering-replacement order a pass keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    /// Keep only what a card did to `copy_of`.
    CopyOnly,
    /// Keep everything a card did except changing `copy_of`, which is settled.
    EverythingElse,
}

fn run_entering_pass(
    state: &mut GameState,
    entering: EnteringPermanent,
    registry: &CardRegistry,
    pass: Pass,
) -> Option<EnteringPermanent> {
    let mut candidates: Vec<(ObjectId, CardId)> = state
        .objects
        .values()
        .filter(|o| {
            registry
                .get(o.card_id)
                .is_some_and(|b| b.replacement_zones().contains(&o.zone))
        })
        .map(|o| (o.id, o.card_id))
        .collect();
    candidates.sort_by_key(|(id, _)| id.0);

    // The entering permanent's own arrival abilities, wherever it currently
    // is — see `apply`'s note. On the second pass those abilities are the
    // copied card's, if a copy effect decided one: a Grimgrin entering as an
    // Essence does not have "enters tapped", because it is not a Grimgrin.
    // On the second pass the entering permanent's own arrival abilities are
    // the *copied* permanent's, if a copy effect decided one: a Grimgrin
    // entering as an Essence does not have "enters tapped", because it is
    // not a Grimgrin. `copy_of` names the permanent, so its card is what
    // those abilities are read from.
    let own = match (pass, entering.copy_of) {
        (Pass::EverythingElse, Some(copied)) => state.get_object(copied).map(|o| o.card_id),
        _ => state.get_object(entering.object).map(|o| o.card_id),
    };
    if let Some(card_id) = own {
        candidates.retain(|(id, _)| *id != entering.object);
        candidates.insert(0, (entering.object, card_id));
    }

    let mut current = EnteringPermanent { ..entering };
    for (object, card_id) in candidates {
        let Some(behavior) = registry.get(card_id) else { continue };
        let asked = ReplaceableEvent::EntersBattlefield(current.clone());
        let Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(next))) =
            behavior.replace_event(state, object, &asked, registry)
        else { continue };
        current = match pass {
            // Only the copy decision survives this pass.
            Pass::CopyOnly => EnteringPermanent { copy_of: next.copy_of, ..current },
            // And on the second, everything but it.
            Pass::EverythingElse => EnteringPermanent { copy_of: current.copy_of, ..next },
        };
    }
    Some(current)
}

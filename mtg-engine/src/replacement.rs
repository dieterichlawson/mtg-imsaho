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
    /// The card it enters as a copy of (CR 706.9), if any.
    pub copy_of: Option<CardId>,
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
/// No board in this card pool can produce two applicable to the same event,
/// so candidates are taken in a deterministic order (by object id) and there
/// is no prompt. The loop is the place to add one.
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

    // A permanent's replacement effect about its OWN arrival applies wherever
    // it currently is — "enters with a +1/+1 counter for each Zombie card in
    // your graveyard" is part of how this object enters, not something the
    // battlefield does to it. And these are evaluated before the zone change
    // (CR 616.1), so at this moment the object is still in the graveyard, the
    // hand, or the library. Without this, Unbreathing Horde reanimated from
    // the graveyard entered with no counters at all.
    if let ReplaceableEvent::EntersBattlefield(e) = &event {
        if !candidates.iter().any(|(id, _)| *id == e.object) {
            if let Some(card_id) = state.get_object(e.object).map(|o| o.card_id) {
                candidates.insert(0, (e.object, card_id));
            }
        }
    }

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
    let own = match (pass, entering.copy_of) {
        (Pass::EverythingElse, Some(copied)) => Some(copied),
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

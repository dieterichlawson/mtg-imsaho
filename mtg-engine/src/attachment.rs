//! Whether an Aura or Equipment on the battlefield is legally attached
//! right now (CR 704.5m/n).
//!
//! One predicate, asked by both sides of the question. `sba.rs` asks it to
//! decide what to sweep; `invariants::permanents` asks it to decide whether
//! a settled board is corrupt. Those used to be two independent readings of
//! the same rules and they disagreed: the checker reported five states as
//! violations that the state-based actions reported nothing to do about, so
//! a board the engine called settled was a board the fuzzer's oracle called
//! broken (issue #552). Sharing the predicate is what stops them being able
//! to disagree — a case added here is swept and checked in the same commit.
//!
//! What "legally attached" means, per CR 704.5m/n, is four questions and not
//! one: is it attached to anything at all, is the host still on the
//! battlefield, is the host a thing this attachment may be attached to, and
//! does the host have protection from it. The engine used to ask only the
//! second.

use crate::cards::{CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::Zone;

/// Why an Aura or Equipment on the battlefield is not legally attached.
///
/// The variants are the reasons CR 704.5m/n name, kept apart because the
/// invariant checker reports them with different words and because a reader
/// of a log line wants to know which one happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Illegality {
    /// Attached to neither an object nor a player. Legal for an Equipment
    /// (CR 301.5c), never for an Aura (CR 704.5m).
    Unattached,
    /// The host object is no longer on the battlefield (CR 704.5m/n). The
    /// one case the engine already handled.
    HostGone,
    /// The host is not something this attachment may be attached to: an
    /// "enchant creature" Aura on a land, an Equipment on a noncreature
    /// (CR 303.4/301.5c), an "enchant player" Aura on an object (CR 702.5d).
    HostIneligible,
    /// The host has protection from it (CR 702.16c/d).
    HostProtected,
    /// Attached to a player it may not be attached to: an Equipment attached
    /// to a player at all (CR 301.5), an Aura on a player who cannot be
    /// enchanted by it (CR 702.16c), or an Aura that enchants objects.
    PlayerIneligible,
}

/// Whether `id` is an attachment at all — an Aura or an Equipment. Anything
/// else has no attachment legality to speak of, and `illegality` says so by
/// returning `None` for it.
#[must_use]
pub fn is_attachment(state: &GameState, id: ObjectId, registry: &CardRegistry) -> bool {
    state.has_subtype(id, "Aura", registry) || state.is_equipment(id, registry)
}

/// Why `id` is illegally attached, or `None` if it is legally attached (or
/// is not an attachment on the battlefield at all).
///
/// Answers the CR 704.5m/n question and nothing else: it does not care
/// whether the attachment is also a creature (CR 303.4d), which is a
/// separate corruption the checker reports on its own.
#[must_use]
pub fn illegality(
    state: &GameState,
    id: ObjectId,
    registry: &CardRegistry,
) -> Option<Illegality> {
    let obj = state.get_object(id)?;
    if obj.zone != Zone::Battlefield {
        return None;
    }
    let is_aura = state.has_subtype(id, "Aura", registry);
    let is_equipment = state.is_equipment(id, registry);
    if !is_aura && !is_equipment {
        return None;
    }

    // An object host and a player host are mutually exclusive; a permanent
    // carrying both is its own corruption, reported by `invariants::mod`.
    if let Some(host) = obj.attached_to {
        if !state.get_object(host).is_some_and(|h| h.zone == Zone::Battlefield) {
            return Some(Illegality::HostGone);
        }
        if !host_is_eligible(state, id, host, is_equipment, registry) {
            return Some(Illegality::HostIneligible);
        }
        // CR 702.16c/d: last, so a host that is the wrong kind reads as the
        // wrong kind rather than as a protection question about it.
        if state.has_protection_from(host, id, registry) {
            return Some(Illegality::HostProtected);
        }
        return None;
    }

    if let Some(player) = obj.attached_to_player {
        // CR 301.5: an Equipment is attached to a creature or to nothing.
        if is_equipment {
            return Some(Illegality::PlayerIneligible);
        }
        if (player.0 as usize) >= state.players.len() {
            return Some(Illegality::PlayerIneligible);
        }
        // CR 702.5d: "enchant creature" does not enchant a player.
        if !enchants_players(state, id, registry) {
            return Some(Illegality::PlayerIneligible);
        }
        // CR 702.16c: protection from the Aura's colour.
        if !state.player_can_be_enchanted_by(id, player, registry) {
            return Some(Illegality::PlayerIneligible);
        }
        return None;
    }

    // CR 704.5m: an Aura attached to nothing goes to its owner's graveyard.
    // CR 301.5c: an Equipment attached to nothing is a perfectly ordinary
    // permanent and stays exactly where it is.
    if is_aura {
        Some(Illegality::Unattached)
    } else {
        None
    }
}

/// Whether this Aura's enchant ability names players rather than objects.
fn enchants_players(state: &GameState, aura: ObjectId, registry: &CardRegistry) -> bool {
    let Some(obj) = state.get_object(aura) else { return false };
    matches!(
        registry.get(obj.card_id).map(super::cards::CardBehavior::target_requirement),
        Some(TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly)
    )
}

/// Whether `host` is something `id` may be attached to (CR 303.4/301.5c),
/// protection aside.
fn host_is_eligible(
    state: &GameState,
    id: ObjectId,
    host: ObjectId,
    is_equipment: bool,
    registry: &CardRegistry,
) -> bool {
    // CR 301.5c: "equipped creature" — an Equipment attaches to creatures.
    if is_equipment {
        return state.is_creature(host, registry);
    }
    let Some(obj) = state.get_object(id) else { return true };
    let Some(behavior) = registry.get(obj.card_id) else { return true };
    match behavior.target_requirement() {
        // CR 702.5d: an Aura that enchants a player is not on an object.
        TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly => false,
        TargetRequirement::Creature => state.is_creature(host, registry),
        TargetRequirement::CreatureWithFilter(filter) => {
            state.is_creature(host, registry)
                && state.get_object(host).is_some_and(|h| {
                    crate::engine::matches_target_filter(
                        state, h, &filter, obj.controller, Some(id), registry,
                    )
                })
        }
        // An enchant ability the engine does not model as a target
        // requirement says nothing about the host, so nothing is claimed
        // about it. This is the silent-failure shape the guide warns about,
        // so it errs towards "legal": an Aura is never swept off a host on
        // the strength of a requirement nobody wrote down.
        _ => true,
    }
}

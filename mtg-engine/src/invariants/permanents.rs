//! Attachments and state-triggered abilities once state-based actions have
//! settled (CR 303.4, 301.5, 702.16c/d, 704.5m/n, 603.8).
//!
//! The attachment half of this file reports, it does not decide. Whether an
//! Aura or Equipment is legally attached is `attachment::illegality`, which
//! is the same predicate `sba.rs` sweeps with — so a state the oracle calls
//! corrupt is a state the engine has already cleaned up, and the two cannot
//! disagree about what a settled board looks like. They used to: the sweep
//! asked only whether the host had left the battlefield, and five states
//! this file reports as violations were states the engine reported nothing
//! to do about (issue #552).

use super::{player_ok, Violations};
use crate::attachment::{illegality, Illegality};
use crate::cards::{CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::Zone;

pub(super) fn check_settled(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield {
            continue;
        }
        let id = obj.id;
        let tag = format!("{} (#{})", obj.name, id.0);
        let is_aura = state.has_subtype(id, "Aura", registry);
        let is_equipment = state.has_subtype(id, "Equipment", registry);

        // CR 704.5m/n: the attachment is legally attached right now.
        if let Some(why) = illegality(state, id, registry) {
            v.push(describe(state, registry, &obj.name, id, is_equipment, why));
        }

        // CR 303.4d/301.5c: an attached Aura or Equipment is not a creature.
        if (is_aura || is_equipment) && obj.attached_to.is_some() && state.is_creature(id, registry) {
            v.push(format!("{tag} is a creature attached to something (CR 303.4d/301.5c)"));
        }

        // CR 603.8: at a fixed point no unflagged permanent's state trigger
        // condition is true — the SBA loop would have fired it.
        if !obj.state_trigger_on_stack {
            if let Some(b) = registry.get(obj.card_id) {
                if b.state_trigger_condition(state, id, registry) {
                    v.push(format!("{tag}'s state trigger condition holds but the trigger has not fired (CR 603.8)"));
                }
            }
        }
    }
}

/// Put an `Illegality` into the words a reader of a fuzz failure needs: what
/// the attachment is, what it is on, and which rule that breaks.
fn describe(
    state: &GameState,
    registry: &CardRegistry,
    name: &str,
    id: ObjectId,
    is_equipment: bool,
    why: Illegality,
) -> String {
    let tag = format!("{name} (#{})", id.0);
    let kind = if is_equipment { "Equipment" } else { "Aura" };
    let host = state.get_object(id).and_then(|o| o.attached_to);
    let host_n = host.map_or(0, |h| h.0);
    match why {
        Illegality::Unattached => format!("Aura {} ({name}) on the battlefield unattached", id.0),
        Illegality::HostGone => format!(
            "{kind} {} ({name}) attached to {host_n} which is not on the battlefield", id.0),
        Illegality::HostProtected => format!(
            "{tag} is attached to #{host_n} which has protection from it (CR 702.16c)"),
        Illegality::HostIneligible if is_equipment => format!(
            "Equipment {} ({name}) attached to non-creature {host_n}", id.0),
        Illegality::HostIneligible => match aura_enchants(state, id, registry) {
            Some(Enchants::Players) =>
                format!("{tag} enchants players but is attached to an object (CR 702.5d)"),
            _ => format!(
                "{tag} enchants creatures but is attached to non-creature #{host_n} (CR 704.5m)"),
        },
        Illegality::PlayerIneligible if is_equipment =>
            format!("{tag} is Equipment attached to a player (CR 301.5)"),
        Illegality::PlayerIneligible => {
            let p = state.get_object(id).and_then(|o| o.attached_to_player);
            match (aura_enchants(state, id, registry), p) {
                (Some(Enchants::Creatures), _) =>
                    format!("{tag} enchants creatures but is attached to a player"),
                (_, Some(p)) if !player_ok(state, p) =>
                    format!("{tag} is attached to p{} who is not a player", p.0),
                (_, Some(p)) =>
                    format!("{tag} enchants p{} who cannot be enchanted by it (CR 702.16c)", p.0),
                (_, None) => format!("{tag} is illegally attached to a player"),
            }
        }
    }
}

/// What an Aura's enchant ability names, as far as the engine models it.
enum Enchants {
    Creatures,
    Players,
}

fn aura_enchants(state: &GameState, id: ObjectId, registry: &CardRegistry) -> Option<Enchants> {
    let obj = state.get_object(id)?;
    match registry.get(obj.card_id)?.target_requirement() {
        TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly => Some(Enchants::Players),
        TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_) => {
            Some(Enchants::Creatures)
        }
        _ => None,
    }
}

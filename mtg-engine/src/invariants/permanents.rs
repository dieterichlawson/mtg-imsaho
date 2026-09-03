//! Attachments and state-triggered abilities once state-based actions have
//! settled (CR 303.4, 301.5, 702.16c/d, 704.5m/n, 603.8).

use super::{player_ok, Violations};
use crate::cards::{CardRegistry, TargetRequirement};
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

        // CR 303.4/702.5: an Aura enchants what its enchant ability allows.
        // (The base check already requires an Aura to be attached to
        // something that is there.)
        if is_aura {
            if let Some(b) = registry.get(obj.card_id) {
                match b.target_requirement() {
                    TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly => {
                        if obj.attached_to.is_some() {
                            v.push(format!("{tag} enchants players but is attached to an object (CR 702.5d)"));
                        }
                        if let Some(p) = obj.attached_to_player {
                            if player_ok(state, p) && !state.player_can_be_enchanted_by(id, p, registry) {
                                v.push(format!("{tag} enchants p{} who cannot be enchanted by it (CR 702.16c)", p.0));
                            }
                        }
                    }
                    TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_) => {
                        if obj.attached_to_player.is_some() {
                            v.push(format!("{tag} enchants creatures but is attached to a player"));
                        }
                        if let Some(host) = obj.attached_to {
                            if state.get_object(host).is_some_and(|h| h.zone == Zone::Battlefield)
                                && !state.is_creature(host, registry)
                            {
                                v.push(format!("{tag} enchants creatures but is attached to non-creature #{} (CR 704.5m)", host.0));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // CR 303.4d/301.5c: an attached Aura or Equipment is not a creature.
        if (is_aura || is_equipment) && obj.attached_to.is_some() && state.is_creature(id, registry) {
            v.push(format!("{tag} is a creature attached to something (CR 303.4d/301.5c)"));
        }
        if is_equipment && obj.attached_to_player.is_some() {
            v.push(format!("{tag} is Equipment attached to a player (CR 301.5)"));
        }
        // CR 702.16c/d: nothing is attached to what has protection from it.
        if let Some(host) = obj.attached_to {
            if state.get_object(host).is_some_and(|h| h.zone == Zone::Battlefield)
                && state.has_protection_from(host, id, registry)
            {
                v.push(format!("{tag} is attached to #{} which has protection from it (CR 702.16c)", host.0));
            }
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

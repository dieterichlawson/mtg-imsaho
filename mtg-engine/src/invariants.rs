//! Structural invariants over [`GameState`], for fuzzing.
//!
//! These are properties that hold at every player decision point regardless
//! of what the cards do — an oracle that is independent of any card's
//! implementation, so a violation is an engine bug with no judgment call
//! involved. The runner checks them after every action when run with
//! `--check-invariants`, and the fuzz suite runs seeded random games under
//! them.
//!
//! Two tiers, because decision points come in two kinds:
//!
//! - [`check_core`] holds at *every* decision point, including a choice
//!   raised in the middle of resolving a spell or ability, when state-based
//!   actions have not run since resolution started mutating the state.
//! - [`check_settled`] additionally holds whenever a player receives
//!   priority or a turn-based-action prompt (attackers, blockers,
//!   mulligans), because CR 704.3 has run state-based actions to a fixed
//!   point right before that.

use crate::cards::CardRegistry;
use crate::ids::PlayerId;
use crate::state::{GameState, StackEntry};
use crate::types::{Keyword, Zone};

/// Invariants that hold at every decision point, even mid-resolution.
/// Returns one message per violation; empty means the state is coherent.
#[must_use]
pub fn check_core(state: &GameState, _registry: &CardRegistry) -> Vec<String> {
    let mut v = Vec::new();

    // Object identity: the map key is the object's id, and the id allocator
    // is ahead of every id it has handed out.
    for obj in state.objects_in_id_order() {
        if !state.objects.get(&obj.id).is_some_and(|o| std::ptr::eq(o, obj)) {
            v.push(format!("object {} not stored under its own id", obj.id.0));
        }
        if obj.id.0 >= state.next_object_id {
            v.push(format!(
                "object {} at or past next_object_id {}",
                obj.id.0, state.next_object_id
            ));
        }
    }

    // Players are indexed by their own ids.
    for (i, p) in state.players.iter().enumerate() {
        if p.id != PlayerId(i as u8) {
            v.push(format!("players[{}] has id {}", i, p.id.0));
        }
    }
    let n_players = state.players.len() as u8;
    if state.active_player.0 >= n_players {
        v.push(format!("active_player {} out of range", state.active_player.0));
    }
    if let Some(pp) = state.priority_player {
        if pp.0 >= n_players {
            v.push(format!("priority_player {} out of range", pp.0));
        }
    }

    // A library's order and the objects in the library zone are the same set,
    // in both directions, with no duplicates (CR 401.2 — a library is an
    // ordered zone; an id listed without an object is a card that is drawn
    // and isn't there, an object without a listing is a card that can never
    // be drawn).
    for p in &state.players {
        let mut seen = std::collections::HashSet::new();
        for &id in &p.library_order {
            if !seen.insert(id) {
                v.push(format!("p{}: {} listed twice in library_order", p.id.0, id.0));
            }
            match state.get_object(id) {
                None => v.push(format!("p{}: library_order lists missing object {}", p.id.0, id.0)),
                Some(o) => {
                    if o.zone != Zone::Library {
                        v.push(format!("p{}: library_order lists {} but its zone is {:?}", p.id.0, id.0, o.zone));
                    }
                    if o.owner != p.id {
                        v.push(format!("p{}: library_order lists {} owned by p{}", p.id.0, id.0, o.owner.0));
                    }
                }
            }
        }
        for obj in state.objects_in_id_order() {
            if obj.zone == Zone::Library && obj.owner == p.id && !seen.contains(&obj.id) {
                v.push(format!("p{}: {} ({}) in library zone but not in library_order", p.id.0, obj.id.0, obj.name));
            }
        }
    }

    // Nothing is attached to a permanent and to a player at once.
    for obj in state.objects_in_id_order() {
        if obj.attached_to.is_some() && obj.attached_to_player.is_some() {
            v.push(format!("{} ({}) attached to both an object and a player", obj.id.0, obj.name));
        }
    }

    // Every spell entry on the stack is an object in the stack zone, and
    // every object in the stack zone is accounted for: on the stack, the
    // spell currently resolving, or a cast still being paid for.
    for entry in &state.stack {
        if let StackEntry::Spell(id) = entry {
            match state.get_object(*id) {
                None => v.push(format!("stack entry Spell({}) has no object", id.0)),
                Some(o) if o.zone != Zone::Stack => {
                    v.push(format!("stack entry Spell({}) is in zone {:?}", id.0, o.zone));
                }
                Some(_) => {}
            }
        }
    }
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Stack {
            continue;
        }
        let on_stack = state.stack.iter().any(|e| e.as_spell() == Some(obj.id));
        let resolving = state.resolving_spell == Some(obj.id);
        let being_cast = state.pending_spell_cast.as_ref().is_some_and(|c| c.object_id == obj.id);
        if !on_stack && !resolving && !being_cast {
            v.push(format!("{} ({}) in stack zone but on no stack entry", obj.id.0, obj.name));
        }
    }

    // Combat bookkeeping only ever names declared attackers. (Dead attackers
    // stay in the maps as snapshots; these are subset checks, not liveness
    // checks.)
    if let Some(combat) = &state.combat {
        for id in &combat.blocked_attackers {
            if !combat.attackers.contains_key(id) {
                v.push(format!("blocked_attackers holds {} which never attacked", id.0));
            }
        }
        for id in combat.blocker_assignments.keys() {
            if !combat.attackers.contains_key(id) {
                v.push(format!("blocker_assignments holds {} which never attacked", id.0));
            }
        }
        for id in combat.planeswalker_defenders.keys() {
            if !combat.attackers.contains_key(id) {
                v.push(format!("planeswalker_defenders holds {} which never attacked", id.0));
            }
        }
    }

    v
}

/// Invariants that additionally hold when state-based actions have just run
/// to a fixed point (CR 704.3) — i.e. at every priority or turn-based-action
/// prompt, but not mid-resolution. Includes everything in [`check_core`].
#[must_use]
pub fn check_settled(state: &GameState, registry: &CardRegistry) -> Vec<String> {
    let mut v = check_core(state, registry);

    // CR 704.5d: a token anywhere but the battlefield has ceased to exist.
    // (The stack is allowed for a token copy of a spell.)
    for obj in state.objects_in_id_order() {
        if obj.is_token && obj.zone != Zone::Battlefield && obj.zone != Zone::Stack {
            v.push(format!("token {} ({}) still exists in {:?}", obj.id.0, obj.name, obj.zone));
        }
    }

    // CR 400.7: leaving the battlefield made it a new object, so
    // battlefield-only markings are gone.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Battlefield {
            continue;
        }
        if obj.tapped {
            v.push(format!("{} ({}) tapped in {:?}", obj.id.0, obj.name, obj.zone));
        }
        if obj.damage_marked != 0 {
            v.push(format!("{} ({}) has {} damage marked in {:?}", obj.id.0, obj.name, obj.damage_marked, obj.zone));
        }
        if obj.attached_to.is_some() || obj.attached_to_player.is_some() {
            v.push(format!("{} ({}) still attached in {:?}", obj.id.0, obj.name, obj.zone));
        }
    }

    // CR 704.5a/b/c: loss conditions have been applied.
    for p in &state.players {
        if p.life <= 0 && !p.lost {
            v.push(format!("p{} at {} life has not lost", p.id.0, p.life));
        }
        if p.has_drawn_from_empty && !p.lost {
            v.push(format!("p{} drew from an empty library and has not lost", p.id.0));
        }
    }

    // CR 704.5f/g/h: a creature that should be dead is dead. Marked damage at
    // or past toughness kills anything not indestructible; zero or less
    // toughness kills even that.
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield || !state.is_creature(obj.id, registry) {
            continue;
        }
        let Some(toughness) = state.effective_toughness(obj.id, registry) else { continue };
        if toughness <= 0 {
            v.push(format!("{} ({}) alive at toughness {}", obj.id.0, obj.name, toughness));
        } else if obj.damage_marked as i32 >= toughness
            && !state.has_keyword(obj.id, Keyword::Indestructible, registry)
        {
            v.push(format!(
                "{} ({}) alive with {} damage on toughness {}",
                obj.id.0, obj.name, obj.damage_marked, toughness
            ));
        }
    }

    // CR 704.5m/n: an Aura on the battlefield is attached, and attached to
    // something that is there.
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield || !state.has_subtype(obj.id, "Aura", registry) {
            continue;
        }
        match (obj.attached_to, obj.attached_to_player) {
            (None, None) => {
                v.push(format!("Aura {} ({}) on the battlefield unattached", obj.id.0, obj.name));
            }
            (Some(host), None) => {
                if !state.get_object(host).is_some_and(|h| h.zone == Zone::Battlefield) {
                    v.push(format!("Aura {} ({}) attached to {} which is not on the battlefield", obj.id.0, obj.name, host.0));
                }
            }
            _ => {}
        }
    }

    // Equipment is attached to a battlefield permanent or not attached at
    // all — unlike an Aura it may sit unattached, but never on a ghost.
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield || !state.has_subtype(obj.id, "Equipment", registry) {
            continue;
        }
        if let Some(host) = obj.attached_to {
            if !state.get_object(host).is_some_and(|h| h.zone == Zone::Battlefield) {
                v.push(format!("Equipment {} ({}) attached to {} which is not on the battlefield", obj.id.0, obj.name, host.0));
            }
        }
    }

    v
}

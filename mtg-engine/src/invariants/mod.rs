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
//! - [`check_transition`] looks at two consecutive decision points and the
//!   action chosen between them: what may not change, what must, and that
//!   every change announced itself in `events`.
//! - [`check_legal`] looks at the legal action set offered at a decision
//!   point: nothing the rules forbid is on the menu, and what they require
//!   is.

use crate::cards::CardRegistry;
use crate::ids::PlayerId;
use crate::state::{GameState, StackEntry};
use crate::types::{Keyword, Zone};

mod effects;
mod events;
mod legal;
mod objects;
mod permanents;
mod prompts;
mod stack;
mod transition;
mod turn;

pub use legal::check_legal;
pub use transition::check_transition;

/// One message per violation.
pub type Violations = Vec<String>;

/// Whether `p` indexes a player of this game (every `get_player` panics
/// otherwise, so this is checked before anything reads through an id).
fn player_ok(state: &GameState, p: PlayerId) -> bool {
    (p.0 as usize) < state.players.len()
}

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

    // Event processing never runs ahead of the events that exist, and no
    // battlefield stamp names a turn that hasn't happened.
    if state.trigger_event_index > state.events.len() {
        v.push(format!(
            "trigger_event_index {} past the {} events that exist",
            state.trigger_event_index, state.events.len()
        ));
    }
    for obj in state.objects_in_id_order() {
        if let Some(t) = obj.attacked_on_turn {
            if t > state.turn_number {
                v.push(format!("{} ({}) attacked on future turn {}", obj.id.0, obj.name, t));
            }
        }
    }

    // Nothing in this pool uses the command zone; an object landing there is
    // a move to the wrong zone, not a mechanic.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Command {
            v.push(format!("{} ({}) in the unused command zone", obj.id.0, obj.name));
        }
    }

    // The attachment graph is acyclic: nothing attached to itself, and no
    // Equipment/Aura ring where following `attached_to` never reaches a
    // host that stands on its own.
    for obj in state.objects_in_id_order() {
        let mut seen = std::collections::HashSet::new();
        let mut cur = obj.id;
        while let Some(next) = state.get_object(cur).and_then(|o| o.attached_to) {
            if !seen.insert(cur) {
                v.push(format!("{} ({}) sits in an attachment cycle", obj.id.0, obj.name));
                break;
            }
            cur = next;
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

        // CR 509.1b: a creature blocks at most one attacker, once. A blocker
        // listed under two attackers (or twice under one) is two blocks from
        // one creature — the shape of the duplicate-declaration family
        // (issue #108 was the attacker side of it).
        let mut blocker_of = std::collections::HashMap::new();
        for (&attacker, blockers) in &combat.blocker_assignments {
            let mut here = std::collections::HashSet::new();
            for &b in blockers {
                if !here.insert(b) {
                    v.push(format!(
                        "blocker {} listed twice against attacker {}", b.0, attacker.0));
                }
                if let Some(prev) = blocker_of.insert(b, attacker) {
                    if prev != attacker {
                        v.push(format!(
                            "blocker {} assigned to attackers {} and {} at once (CR 509.1b)",
                            b.0, prev.0, attacker.0));
                    }
                }
            }
        }
    }

    // The events of the CURRENT action (submit_action clears the buffer per
    // action) are an oracle too — triggers fire once per event, so a
    // malformed event multiplies effects even when the state maps look fine.
    //
    // CR 508.1a/508.2: declaring attackers chooses a SET; an id repeated in
    // AttackersDeclared fires the creature's attack trigger once per repeat.
    // Combat's own maps dedupe, so this was invisible to every state check
    // while tripling Kessig Cagebreakers' wolves (issue #108).
    for e in &state.events {
        if let crate::events::GameEvent::AttackersDeclared { attackers } = e {
            let mut seen = std::collections::HashSet::new();
            for (id, _) in attackers {
                if !seen.insert(*id) {
                    v.push(format!("AttackersDeclared lists attacker {} more than once", id.0));
                }
            }
        }
    }

    // Every life transition goes through change_life, which records a
    // LifeChanged event — so within one action's events, each player's
    // LifeChanged chain must link up (old == the previous new) and the last
    // link must equal the player's actual life. A break means a life total
    // moved without the event (and, since #129, without the log line) —
    // the unlogged-life-change family, mechanically checked.
    {
        let mut last_new: std::collections::HashMap<PlayerId, i32> =
            std::collections::HashMap::new();
        for e in &state.events {
            if let crate::events::GameEvent::LifeChanged { player, old, new_life } = e {
                if let Some(prev) = last_new.get(player) {
                    if prev != old {
                        v.push(format!(
                            "p{}: LifeChanged chain breaks ({} -> event starting at {})",
                            player.0, prev, old));
                    }
                }
                last_new.insert(*player, *new_life);
            }
        }
        for (p, n) in last_new {
            if p.0 < n_players && state.get_player(p).life != n {
                v.push(format!(
                    "p{}: last LifeChanged says {} but life is {}",
                    p.0, n, state.get_player(p).life));
            }
        }
    }

    // A pending prompt is answerable and its stashes match. A choice with
    // nothing to choose is a stuck game; an X-funding prompt whose stashed
    // cast/activation is missing panics when answered; a stash with no
    // prompt is a leak (the #123 cancel path clears both together).
    match &state.awaiting_action {
        Some(crate::state::AwaitingAction::ResolutionChoice { player, choice, .. }) => {
            if player.0 >= n_players {
                v.push(format!("awaiting_action prompts out-of-range p{}", player.0));
            }
            use crate::state::ResolutionChoiceKind as K;
            let empty = match choice {
                K::ChooseTarget { options, .. } => options.is_empty(),
                K::ChooseFromLookedAt { looked_at, .. } => looked_at.is_empty(),
                K::ChooseCardFromHand { cards, .. } => cards.is_empty(),
                K::ChooseTriggerOrder { options, .. } => options.is_empty(),
                K::DividePermanentsIntoPiles { permanents, .. } => permanents.is_empty(),
                _ => false,
            };
            if empty {
                v.push("awaiting_action offers a choice with nothing to choose".into());
            }
            match choice {
                K::ChooseXFunding { is_ability: false, .. } if state.pending_spell_cast.is_none() => {
                    v.push("spell X-funding prompt with no pending_spell_cast stashed".into());
                }
                K::ChooseXFunding { is_ability: true, .. } if state.pending_ability_effect.is_none() => {
                    v.push("ability X-funding prompt with no pending_ability_effect stashed".into());
                }
                _ => {}
            }
        }
        Some(crate::state::AwaitingAction::MulliganDecision { player })
        | Some(crate::state::AwaitingAction::BottomAfterMulligan { player, .. })
        | Some(crate::state::AwaitingAction::DiscardToHandSize { player, .. })
            if player.0 >= n_players =>
        {
            v.push(format!("awaiting_action prompts out-of-range p{}", player.0));
        }
        _ => {}
    }
    {
        let awaiting_stashes_spell = matches!(&state.awaiting_action,
            Some(crate::state::AwaitingAction::ResolutionChoice {
                choice: crate::state::ResolutionChoiceKind::ChooseXFunding { is_ability: false, .. }
                    | crate::state::ResolutionChoiceKind::ChooseExileFromGraveyard { .. },
                ..
            }));
        if state.pending_spell_cast.is_some() && !awaiting_stashes_spell {
            v.push("pending_spell_cast stashed with no funding/exile prompt up (leak)".into());
        }
        let awaiting_stashes_ability = matches!(&state.awaiting_action,
            Some(crate::state::AwaitingAction::ResolutionChoice {
                choice: crate::state::ResolutionChoiceKind::ChooseXFunding { is_ability: true, .. },
                ..
            }));
        if state.pending_ability_effect.is_some() && !awaiting_stashes_ability {
            v.push("pending_ability_effect stashed with no funding prompt up (leak)".into());
        }
    }

    objects::check_core(state, _registry, &mut v);
    stack::check_core(state, _registry, &mut v);
    prompts::check_core(state, _registry, &mut v);
    turn::check_core(state, _registry, &mut v);
    events::check_core(state, _registry, &mut v);
    effects::check_core(state, _registry, &mut v);

    v
}

/// Invariants that additionally hold when state-based actions have just run
/// to a fixed point (CR 704.3) — i.e. at every priority or turn-based-action
/// prompt, but not mid-resolution. Includes everything in [`check_core`].
#[must_use]
pub fn check_settled(state: &GameState, registry: &CardRegistry) -> Vec<String> {
    let mut v = check_core(state, registry);

    // CR 704.5d: a token anywhere but the battlefield has ceased to exist.
    // (Nothing in this pool copies a spell, so no token is ever on the
    // stack either.)
    for obj in state.objects_in_id_order() {
        if obj.is_token && obj.zone != Zone::Battlefield {
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
        if !obj.counters.is_empty() {
            v.push(format!("{} ({}) still has counters in {:?}", obj.id.0, obj.name, obj.zone));
        }
        if obj.regeneration_shields != 0 {
            v.push(format!("{} ({}) keeps a regeneration shield in {:?}", obj.id.0, obj.name, obj.zone));
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
    //
    // Exception: a permanent whose "enters as a copy" choice is still being
    // made (Evil Twin). The engine models CR 614.1d as a brief window where
    // the printed 0/0 sits on the battlefield exempt from state-based
    // actions until the copy choice concludes — `entering_copy_source` is
    // that exemption, and the checker honors it exactly as sba.rs does. The
    // window cannot outlive the turn the permanent entered, so a guard still
    // armed on a creature past its summoning sickness is a leak, not a
    // window — that gets its own violation below.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Battlefield && obj.entering_copy_source && !obj.summoning_sick {
            v.push(format!(
                "{} ({}) still exempt from SBAs long after its copy-entry window",
                obj.id.0, obj.name
            ));
        }
    }
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield
            || !state.is_creature(obj.id, registry)
            || obj.entering_copy_source
        {
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

    // Combat state exists only during the steps that use it: it is created
    // when attackers are declared and cleared as the end-of-combat step
    // begins.
    if state.combat.is_some()
        && !matches!(state.step,
            crate::types::Step::DeclareAttackers
            | crate::types::Step::DeclareBlockers
            | crate::types::Step::CombatDamage)
    {
        v.push(format!("combat state present in step {:?}", state.step));
    }

    // CR 508.1a / 506.4d: an attacker still on the battlefield is controlled
    // by the active player — a creature whose controller changed left combat
    // the moment it changed hands. The same for a live blocker and the
    // defending player of the attack it blocks.
    if let Some(combat) = &state.combat {
        for (&attacker, &defender) in &combat.attackers {
            if let Some(o) = state.get_object(attacker) {
                if o.zone == Zone::Battlefield && o.controller != state.active_player {
                    v.push(format!(
                        "attacker {} ({}) is controlled by p{}, not the active player",
                        attacker.0, o.name, o.controller.0
                    ));
                }
            }
            for &blocker in combat.blocker_assignments.get(&attacker).into_iter().flatten() {
                if let Some(b) = state.get_object(blocker) {
                    if b.zone == Zone::Battlefield && b.controller != defender {
                        v.push(format!(
                            "blocker {} ({}) is controlled by p{}, not the defending player p{}",
                            blocker.0, b.name, b.controller.0, defender.0
                        ));
                    }
                }
            }
        }
    }

    // CR 603.3b: every triggered ability that has been collected is on the
    // stack (or waiting on a choice) before any player holds priority.
    if state.awaiting_action.is_none() && state.priority_player.is_some() {
        let waiting = state.pending_triggers.len()
            + state.pending_trigger_pushes_ap.len()
            + state.pending_trigger_pushes_nap.len();
        if waiting != 0 {
            v.push(format!(
                "{waiting} collected trigger(s) still queued while a player holds priority"
            ));
        }
    }

    // A battlefield creature has a power and a toughness — state-based
    // actions compare against them, so a creature with neither is one the
    // death rules cannot see.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Battlefield && state.is_creature(obj.id, registry) {
            if state.effective_power(obj.id, registry).is_none()
                || state.effective_toughness(obj.id, registry).is_none()
            {
                v.push(format!("creature {} ({}) has no power/toughness", obj.id.0, obj.name));
            }
        }
    }

    // Only Auras and Equipment attach to objects, and only planeswalkers
    // carry loyalty.
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield {
            continue;
        }
        if obj.attached_to.is_some()
            && !state.has_subtype(obj.id, "Aura", registry)
            && !state.has_subtype(obj.id, "Equipment", registry)
        {
            v.push(format!("{} ({}) attached to an object but is no Aura or Equipment", obj.id.0, obj.name));
        }
        if obj.counters.get(&crate::types::CounterType::Loyalty).copied().unwrap_or(0) > 0
            && !state.has_card_type(obj.id, crate::types::CardType::Planeswalker, registry)
        {
            v.push(format!("{} ({}) holds loyalty counters but is no planeswalker", obj.id.0, obj.name));
        }
    }

    // CR 704.5h: a creature dealt damage by a deathtouch source since the
    // last SBA check is destroyed, however small the damage. Regeneration
    // clears the marked damage, so a saved creature doesn't trip this.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Battlefield
            && state.is_creature(obj.id, registry)
            && obj.dealt_deathtouch_damage
            && obj.damage_marked > 0
            && !obj.entering_copy_source
            && !state.has_keyword(obj.id, Keyword::Indestructible, registry)
        {
            v.push(format!(
                "{} ({}) alive with deathtouch damage marked on it",
                obj.id.0, obj.name
            ));
        }
    }

    // CR 704.5i: a planeswalker with 0 loyalty is in its owner's graveyard,
    // not on the battlefield.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Battlefield
            && state.has_card_type(obj.id, crate::types::CardType::Planeswalker, registry)
            && obj.counters.get(&crate::types::CounterType::Loyalty).copied().unwrap_or(0) == 0
        {
            v.push(format!("planeswalker {} ({}) alive at 0 loyalty", obj.id.0, obj.name));
        }
    }

    // CR 704.5j: no player controls two legendary permanents with the same
    // name once state-based actions have settled. The keep-choice is itself
    // a prompt, and the legend rule holds off while another prompt is
    // pending — so this is only claimable when nothing is being asked.
    if state.awaiting_action.is_none() {
        let mut seen = std::collections::HashSet::new();
        for obj in state.objects_in_id_order() {
            if obj.zone != Zone::Battlefield || !state.is_legendary(obj.id, registry) {
                continue;
            }
            let name = state.name_of(obj.id, registry);
            if !seen.insert((obj.controller, name.clone())) {
                v.push(format!(
                    "p{} controls two legendary permanents named {}",
                    obj.controller.0, name
                ));
            }
        }
    }

    // CR 704.5n: +1/+1 and -1/-1 counters annihilate in pairs; nothing
    // holds both once SBAs have settled.
    for obj in state.objects_in_id_order() {
        if obj.zone == Zone::Battlefield
            && obj.counters.get(&crate::types::CounterType::PlusOnePlusOne).copied().unwrap_or(0) > 0
            && obj.counters.get(&crate::types::CounterType::MinusOneMinusOne).copied().unwrap_or(0) > 0
        {
            v.push(format!(
                "{} ({}) holds both +1/+1 and -1/-1 counters",
                obj.id.0, obj.name
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

    // Equipment is attached to a battlefield creature or not attached at
    // all — unlike an Aura it may sit unattached, but never on a ghost, and
    // never on a non-creature (CR 704.5p unattaches it).
    for obj in state.objects_in_id_order() {
        if obj.zone != Zone::Battlefield || !state.has_subtype(obj.id, "Equipment", registry) {
            continue;
        }
        if let Some(host) = obj.attached_to {
            if !state.get_object(host).is_some_and(|h| h.zone == Zone::Battlefield) {
                v.push(format!("Equipment {} ({}) attached to {} which is not on the battlefield", obj.id.0, obj.name, host.0));
            } else if !state.is_creature(host, registry) {
                v.push(format!("Equipment {} ({}) attached to non-creature {}", obj.id.0, obj.name, host.0));
            }
        }
    }


    turn::check_settled(state, registry, &mut v);
    effects::check_settled(state, registry, &mut v);
    permanents::check_settled(state, registry, &mut v);

    v
}

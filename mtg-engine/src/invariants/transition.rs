//! Invariants over a *transition*: the state at one decision point, the
//! action chosen there, and the state at the next decision point.
//!
//! A single state cannot say whether a counter moved the way the rules
//! allow, whether an object that changed zones announced it, or whether a
//! pass did nothing. The pair can. `cur.events` holds the events of the last
//! `submit_action`; a transition covers exactly one submit iff
//! `cur.submit_seq == prev.submit_seq + 1` (the game loop may auto-pass for
//! a player with nothing to do, which is a second submit with its own
//! buffer), and only then are the event-ledger clauses applied.

use super::{player_ok, Violations};
use crate::actions::{Action, CombatPrompt, ResolvedChoice, Target};
use crate::cards::CardRegistry;
use crate::events::{DamageTarget, GameEvent, LossReason};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameObject, GameState, PendingEffect, ResolutionChoiceKind, StackEntry};
use crate::triggers::{PendingTrigger, TriggerEvent};
use crate::types::{ManaType, Step, Zone};
use std::collections::{BTreeSet, HashMap};

/// One message per violation of the pair (`prev` → `cur`) reached by
/// `action` (None when the action is not known, e.g. after a resume).
#[must_use]
pub fn check_transition(prev: &GameState, action: Option<&Action>, cur: &GameState, registry: &CardRegistry) -> Violations {
    let mut v = Vec::new();
    let single = cur.submit_seq == prev.submit_seq + 1;
    let events: &[GameEvent] = if single { &cur.events } else { &[] };
    let same_turn = cur.turn_number == prev.turn_number;
    let stayed = |id: ObjectId| -> bool {
        match (prev.get_object(id), cur.get_object(id)) {
            (Some(a), Some(b)) => a.zone_change_count == b.zone_change_count,
            _ => false,
        }
    };

    identity(prev, cur, &mut v);
    monotone(prev, cur, &mut v);
    per_turn(prev, cur, same_turn, single, events, &mut v);
    walk(prev, cur, single, events, &mut v);
    if single {
        zone_ledger(prev, cur, events, &mut v);
        status_ledgers(prev, cur, events, &stayed, registry, &mut v);
        life_and_loss(prev, cur, action, events, &mut v);
        mana_ledger(prev, cur, action, events, &mut v);
        if let Some(a) = action {
            action_contract(prev, cur, a, events, &stayed, registry, &mut v);
        }
        triggers_witnessed(prev, cur, events, &mut v);
    }
    v
}

fn printed(o: &GameObject) -> crate::ids::CardId {
    o.copy_grantor.unwrap_or(o.card_id)
}

/// CR 108.3, 400.7, 111.7: owners never change, cards never vanish, new
/// objects are tokens with fresh ids.
fn identity(prev: &GameState, cur: &GameState, v: &mut Violations) {
    for a in prev.objects_in_id_order() {
        match cur.get_object(a.id) {
            None => {
                if !a.is_token {
                    v.push(format!("card #{} ({}) ceased to exist (CR 108.3)", a.id.0, a.name));
                }
            }
            Some(b) => {
                if b.owner != a.owner {
                    v.push(format!("#{} ({}) changed owner p{} -> p{} (CR 108.3)", a.id.0, a.name, a.owner.0, b.owner.0));
                }
                if b.is_token != a.is_token {
                    v.push(format!("#{} ({}) changed token-ness", a.id.0, a.name));
                }
                if printed(b) != printed(a) {
                    v.push(format!("#{} ({}) changed printed card {} -> {} (CR 707.2)", a.id.0, a.name, printed(a).0, printed(b).0));
                }
                if !a.is_token && a.zone_change_count == b.zone_change_count && a.copy_grantor.is_none()
                    && b.copy_grantor.is_none() && a.is_transformed == b.is_transformed && a.name != b.name
                {
                    v.push(format!("#{} was renamed {:?} -> {:?} without a copy or transform", a.id.0, a.name, b.name));
                }
                if b.card_id != a.card_id
                    && !((b.copy_grantor.is_some() && b.zone == Zone::Battlefield)
                        || (a.copy_grantor.is_some() && b.zone_change_count > a.zone_change_count))
                {
                    v.push(format!("#{} ({}) changed card {} -> {} without a copy or a zone change", a.id.0, a.name, a.card_id.0, b.card_id.0));
                }
            }
        }
    }
    for b in cur.objects_in_id_order() {
        if prev.get_object(b.id).is_none() {
            if !b.is_token {
                v.push(format!("card #{} ({}) appeared mid-game (CR 108.3)", b.id.0, b.name));
            }
            if b.id.0 < prev.next_object_id {
                v.push(format!("new object #{} reuses an id below the allocator's {}", b.id.0, prev.next_object_id));
            }
        }
    }
}

/// Counters that only ever grow; records that never revert.
fn monotone(prev: &GameState, cur: &GameState, v: &mut Violations) {
    if cur.next_object_id < prev.next_object_id {
        v.push(format!("next_object_id went back {} -> {}", prev.next_object_id, cur.next_object_id));
    }
    if cur.turn_number < prev.turn_number {
        v.push(format!("turn_number went back {} -> {}", prev.turn_number, cur.turn_number));
    }
    if cur.submit_seq < prev.submit_seq {
        v.push(format!("submit_seq went back {} -> {}", prev.submit_seq, cur.submit_seq));
    }
    for a in prev.objects_in_id_order() {
        if let Some(b) = cur.get_object(a.id) {
            if b.zone_change_count < a.zone_change_count {
                v.push(format!("#{} ({}) zone_change_count went back {} -> {}", a.id.0, a.name, a.zone_change_count, b.zone_change_count));
            }
            if b.zone_change_count == a.zone_change_count && b.zone != a.zone {
                v.push(format!("#{} ({}) moved {:?} -> {:?} without a zone change being counted (CR 400.7)", a.id.0, a.name, a.zone, b.zone));
            }
            if let Some(t) = a.attacked_on_turn {
                if b.zone_change_count == a.zone_change_count
                    && b.attacked_on_turn != Some(t) && b.attacked_on_turn != Some(cur.turn_number)
                {
                    v.push(format!("#{} ({}) forgot attacking on turn {t}", a.id.0, a.name));
                }
            }
        }
    }
    for (pa, pb) in prev.players.iter().zip(&cur.players) {
        if pb.mulligan_count < pa.mulligan_count {
            v.push(format!("p{} mulligan count went back", pa.id.0));
        }
        if pa.mulligan_kept && !pb.mulligan_kept {
            v.push(format!("p{} un-kept their hand", pa.id.0));
        }
        if pa.lost && (!pb.lost || pb.loss_reason != pa.loss_reason) {
            v.push(format!("p{} un-lost the game (CR 104.3)", pa.id.0));
        }
    }
    if prev.result.is_some() && cur.result != prev.result {
        v.push(format!("the result changed {:?} -> {:?} (CR 104.4)", prev.result, cur.result));
    }
    if cur.game_log.len() < prev.game_log.len() {
        v.push("the game log shrank".into());
    } else if let Some(last) = prev.game_log.last() {
        if cur.game_log[prev.game_log.len() - 1].message != last.message {
            v.push("the game log was rewritten".into());
        }
    }
}

/// CR 305.2, 606.3, morbid: per-turn bookkeeping moves only the way the
/// turn allows, and exactly as the events say.
fn per_turn(prev: &GameState, cur: &GameState, same_turn: bool, single: bool, events: &[GameEvent], v: &mut Violations) {
    let spells = |s: &GameState, p: PlayerId| s.num_spells_cast_this_turn.get(&p).copied().unwrap_or(0);
    for (pa, pb) in prev.players.iter().zip(&cur.players) {
        let p = pa.id;
        if same_turn {
            if pb.land_plays_remaining > pa.land_plays_remaining {
                v.push(format!("p{} regained a land drop mid-turn (CR 305.2)", p.0));
            }
            if spells(cur, p) < spells(prev, p) {
                v.push(format!("p{}'s spells-cast-this-turn count went back", p.0));
            }
            if single {
                let played = events.iter().filter(|e| matches!(e, GameEvent::LandPlayed { player, .. } if *player == p)).count() as u32;
                if pb.land_plays_remaining + played != pa.land_plays_remaining {
                    v.push(format!("p{} land drops {} -> {} with {played} LandPlayed (CR 305.2)", p.0, pa.land_plays_remaining, pb.land_plays_remaining));
                }
                let cast = events.iter().filter(|e| matches!(e, GameEvent::SpellCast { player, .. } if *player == p)).count() as u32;
                if spells(cur, p) != spells(prev, p) + cast {
                    v.push(format!("p{} spells cast this turn {} -> {} with {cast} SpellCast", p.0, spells(prev, p), spells(cur, p)));
                }
            }
        } else if single && cur.turn_number == prev.turn_number + 1 {
            let before_turn = events.iter().take_while(|e| !matches!(e, GameEvent::TurnStarted { .. }))
                .filter(|e| matches!(e, GameEvent::SpellCast { player, .. } if *player == p)).count() as u32;
            let last = cur.num_spells_cast_last_turn.get(&p).copied().unwrap_or(0);
            if last != spells(prev, p) + before_turn {
                v.push(format!("p{} cast {} spells last turn but the record says {last}", p.0, spells(prev, p) + before_turn));
            }
        }
    }
    if same_turn && prev.creature_died_this_turn && !cur.creature_died_this_turn {
        v.push("the morbid flag was reset mid-turn".into());
    }
    if single && same_turn && !prev.creature_died_this_turn && cur.creature_died_this_turn
        && !events.iter().any(|e| matches!(e, GameEvent::CreatureDied { .. }))
    {
        v.push("the morbid flag was set with no creature dying".into());
    }
    for b in cur.objects_in_id_order() {
        if b.zone != Zone::Battlefield {
            continue;
        }
        if let Some(a) = prev.get_object(b.id) {
            if same_turn && a.zone_change_count == b.zone_change_count
                && !a.abilities_activated_this_turn.is_subset(&b.abilities_activated_this_turn)
            {
                v.push(format!("#{} ({}) forgot an activation this turn", b.id.0, b.name));
            }
        }
        if !same_turn && !b.abilities_activated_this_turn.is_empty() {
            v.push(format!("#{} ({}) remembers activations from a previous turn", b.id.0, b.name));
        }
    }
}

fn step_index(s: Step) -> usize {
    s as usize
}

/// CR 500.1, 103.7a, 508.8, 510.5, 514.3a: steps advance in order, turns
/// alternate, and every step change was announced.
fn walk(prev: &GameState, cur: &GameState, single: bool, events: &[GameEvent], v: &mut Violations) {
    let d = cur.turn_number.saturating_sub(prev.turn_number);
    if d == 0 && step_index(cur.step) < step_index(prev.step) {
        v.push(format!("step went back {:?} -> {:?} within turn {}", prev.step, cur.step, cur.turn_number));
    }
    if (cur.active_player == prev.active_player) != (d % 2 == 0) {
        v.push(format!("active player p{} -> p{} over {d} turn(s)", prev.active_player.0, cur.active_player.0));
    }
    if !single {
        return;
    }
    let in_mulligan = matches!(prev.awaiting_action, Some(AwaitingAction::MulliganDecision { .. } | AwaitingAction::BottomAfterMulligan { .. }));
    let turns = events.iter().filter(|e| matches!(e, GameEvent::TurnStarted { .. })).count() as u32;
    if in_mulligan {
        if d != 0 || turns > 1 {
            v.push(format!("{turns} turn(s) started and {d} counted while leaving the opening hands"));
        }
    } else if turns != d {
        v.push(format!("{turns} TurnStarted event(s) for a turn counter that moved by {d}"));
    }
    let mut step = prev.step;
    let mut turn = prev.turn_number;
    let mut active = prev.active_player;
    let mut any_step = false;
    for (i, e) in events.iter().enumerate() {
        match e {
            GameEvent::TurnStarted { player, turn: t } => {
                if in_mulligan && *t == 1 {
                    continue;
                }
                if *t != turn + 1 || *player != prev.opponent(active) || step != Step::Cleanup {
                    v.push(format!("TurnStarted {{turn {t}, p{}}} after turn {turn} ({step:?}, p{} active)", player.0, active.0));
                }
                turn = *t;
                active = *player;
            }
            GameEvent::StepStarted { step: s } => {
                let ok = Some(*s) == step.next()
                    || (step == Step::Upkeep && *s == Step::PrecombatMain && turn == 1)
                    || (step == Step::DeclareAttackers && *s == Step::EndCombat)
                    || (step == Step::CombatDamage && *s == Step::CombatDamage)
                    || (step == Step::Cleanup && *s == Step::Cleanup)
                    || (step == Step::Cleanup && *s == Step::Untap
                        && i > 0 && matches!(events[i - 1], GameEvent::TurnStarted { .. }))
                    || (in_mulligan && *s == Step::Untap && !any_step);
                if !ok {
                    v.push(format!("StepStarted {s:?} after {step:?} (CR 500.1)"));
                }
                step = *s;
                any_step = true;
            }
            _ => {}
        }
    }
    if !any_step && (cur.step != prev.step || d != 0) {
        v.push(format!("{:?}/turn {} -> {:?}/turn {} with no StepStarted", prev.step, prev.turn_number, cur.step, cur.turn_number));
    }
    if any_step && step != cur.step {
        v.push(format!("the last StepStarted names {step:?} but the step is {:?}", cur.step));
    }
}

/// CR 400.7: every zone change is announced, in order, from the zone the
/// object was in to the zone it is in; the verbs pair with their moves.
fn zone_ledger(prev: &GameState, cur: &GameState, events: &[GameEvent], v: &mut Violations) {
    let mut moves: HashMap<ObjectId, Vec<(Zone, Zone)>> = HashMap::new();
    for e in events {
        if let GameEvent::ObjectMoved { object, from, to } = e {
            moves.entry(*object).or_default().push((*from, *to));
        }
    }
    for b in cur.objects_in_id_order() {
        let m = moves.get(&b.id).cloned().unwrap_or_default();
        let (from_zone, from_count) = match prev.get_object(b.id) {
            Some(a) => (Some(a.zone), a.zone_change_count),
            None => (None, 0),
        };
        let expected = b.zone_change_count.saturating_sub(from_count) as usize;
        // CR 111.2: a token is created on the battlefield, and says so.
        if from_zone.is_none()
            && !events.iter().any(|e| matches!(e, GameEvent::EnteredBattlefield { object, .. } if *object == b.id))
        {
            v.push(format!("#{} ({}) appeared without entering the battlefield (CR 111.2)", b.id.0, b.name));
        }
        if m.len() != expected {
            v.push(format!("#{} ({}) moved {} time(s) but announced {} (CR 400.7)", b.id.0, b.name, expected, m.len()));
            continue;
        }
        let mut at = from_zone;
        for (from, to) in &m {
            if let Some(z) = at {
                if *from != z {
                    v.push(format!("#{} ({}) announced a move from {from:?} while in {z:?}", b.id.0, b.name));
                }
            }
            at = Some(*to);
        }
        if let Some(z) = at {
            if z != b.zone {
                v.push(format!("#{} ({}) last announced moving to {z:?} but is in {:?}", b.id.0, b.name, b.zone));
            }
        }
    }
    for a in prev.objects_in_id_order() {
        if cur.get_object(a.id).is_none() {
            if let Some(last) = moves.get(&a.id).and_then(|m| m.last()) {
                if last.1 == Zone::Battlefield {
                    v.push(format!("#{} ({}) ceased to exist after moving to the battlefield", a.id.0, a.name));
                }
            }
        }
    }
    let moved = |id: ObjectId, from: Option<Zone>, to: Zone| -> bool {
        moves.get(&id).is_some_and(|m| m.iter().any(|(f, t)| *t == to && from.is_none_or(|z| *f == z)))
    };
    for e in events {
        let (what, id, ok) = match e {
            GameEvent::CardDrawn { object, .. } => ("CardDrawn", *object, moved(*object, Some(Zone::Library), Zone::Hand)),
            GameEvent::Discarded { object, .. } => ("Discarded", *object, moved(*object, Some(Zone::Hand), Zone::Graveyard)),
            GameEvent::CreatureCardMilled { object, .. } => ("CreatureCardMilled", *object, moved(*object, Some(Zone::Library), Zone::Graveyard)),
            GameEvent::SpellCast { object, .. } => ("SpellCast", *object, match cur.get_object(*object) {
                // CR 702.34a: a flashback cast comes from the graveyard; a
                // card may also be castable from there on its own (Skaab
                // Ruinator), so the converse does not hold.
                Some(o) if o.zone == Zone::Stack && o.cast_with_flashback => moved(*object, Some(Zone::Graveyard), Zone::Stack),
                _ => moved(*object, Some(Zone::Hand), Zone::Stack) || moved(*object, Some(Zone::Graveyard), Zone::Stack),
            }),
            GameEvent::LeftBattlefield { object, to, .. } => ("LeftBattlefield", *object, moved(*object, Some(Zone::Battlefield), *to)),
            GameEvent::EnteredBattlefield { object, .. } => ("EnteredBattlefield", *object,
                moved(*object, None, Zone::Battlefield) || prev.get_object(*object).is_none()),
            GameEvent::LandPlayed { object, .. } => ("LandPlayed", *object, moved(*object, Some(Zone::Hand), Zone::Battlefield)),
            _ => continue,
        };
        if !ok {
            v.push(format!("{what} #{} without the matching zone change", id.0));
        }
    }
}

/// Status flips and per-object ledgers that need a witness in the buffer.
fn status_ledgers(prev: &GameState, cur: &GameState, events: &[GameEvent], stayed: &dyn Fn(ObjectId) -> bool,
                  registry: &CardRegistry, v: &mut Violations) {
    let cleanup = events.iter().any(|e| matches!(e, GameEvent::StepStarted { step: Step::Cleanup }));
    let untap = events.iter().any(|e| matches!(e, GameEvent::StepStarted { step: Step::Untap }));
    for b in cur.objects_in_id_order() {
        let Some(a) = prev.get_object(b.id) else { continue };
        if !stayed(b.id) {
            continue;
        }
        let tag = format!("#{} ({})", b.id.0, b.name);
        // CR 701.20a/701.21a: tap and untap are edges with events.
        if b.zone == Zone::Battlefield && a.tapped != b.tapped {
            let want = if b.tapped { "Tapped" } else { "Untapped" };
            let seen = events.iter().any(|e| match e {
                GameEvent::Tapped { object } => b.tapped && *object == b.id,
                GameEvent::Untapped { object } => !b.tapped && *object == b.id,
                _ => false,
            });
            if !seen {
                v.push(format!("{tag} became {} with no {want} event", if b.tapped { "tapped" } else { "untapped" }));
            }
        }
        // CR 120.3, 701.15a, 514.2: marked damage grows by the damage dealt
        // and shrinks only through regeneration or cleanup.
        if b.zone == Zone::Battlefield && !cur.has_card_type(b.id, crate::types::CardType::Planeswalker, registry) {
            let dealt: u32 = events.iter().map(|e| match e {
                GameEvent::CombatDamageDealt { target: DamageTarget::Object(o), amount, .. }
                | GameEvent::NonCombatDamageDealt { target: DamageTarget::Object(o), amount, .. } if *o == b.id => *amount,
                _ => 0,
            }).sum();
            if b.damage_marked > a.damage_marked + dealt {
                v.push(format!("{tag} has {} damage marked after {} + {dealt} dealt (CR 120.3)", b.damage_marked, a.damage_marked));
            }
            let shields_moved = a.regeneration_shields > 0 || b.regeneration_shields != a.regeneration_shields;
            if b.damage_marked < a.damage_marked + dealt && !cleanup && !shields_moved {
                v.push(format!("{tag} lost marked damage ({} + {dealt} -> {}) with no regeneration or cleanup", a.damage_marked, b.damage_marked));
            }
            if !a.dealt_deathtouch_damage && b.dealt_deathtouch_damage && dealt == 0 {
                v.push(format!("{tag} was marked with deathtouch damage that was never dealt"));
            }
        }
        // CR 701.15a: regenerating taps and removes from combat.
        if b.zone == Zone::Battlefield && b.regeneration_shields < a.regeneration_shields && !cleanup {
            if !a.tapped && !events.iter().any(|e| matches!(e, GameEvent::Tapped { object } if *object == b.id)) {
                v.push(format!("{tag} regenerated without tapping (CR 701.15a)"));
            }
            if let Some(c) = &cur.combat {
                if c.attackers.contains_key(&b.id) || c.blocker_assignments.values().flatten().any(|x| *x == b.id) {
                    v.push(format!("{tag} regenerated but is still in combat (CR 701.15a)"));
                }
            }
        }
        // CR 302.6: a change of controller brings summoning sickness.
        if b.zone == Zone::Battlefield {
            if b.controller != a.controller && !b.summoning_sick && !cleanup && !untap {
                v.push(format!("{tag} changed controller p{} -> p{} without summoning sickness (CR 302.6)", a.controller.0, b.controller.0));
            }
            // The converse (sickness implies a change of controller) is not
            // claimed: control can legitimately change away and back inside
            // one action, which breaks continuity without moving the name.
        }
        if b.last_controller != a.last_controller {
            v.push(format!("{tag} rewrote its last controller without leaving"));
        }
        // CR 508.1: an attack stamp comes from a declaration.
        if b.attacked_on_turn != a.attacked_on_turn {
            let declared = events.iter().any(|e| matches!(e, GameEvent::AttackersDeclared { attackers }
                if attackers.iter().any(|(id, _)| *id == b.id)));
            if b.attacked_on_turn != Some(cur.turn_number) || !declared {
                v.push(format!("{tag} was stamped as attacking without a declaration (CR 508.1)"));
            }
        }
    }
    // CR 701.20a: without a shuffle, a library keeps its order (draws and
    // mills take the top, searches shuffle, cards go to the top or bottom).
    for (pa, pb) in prev.players.iter().zip(&cur.players) {
        let p = pa.id;
        if events.iter().any(|e| matches!(e, GameEvent::LibraryShuffled { player } if *player == p)) {
            continue;
        }
        let common: Vec<ObjectId> = pa.library_order.iter().copied().filter(|id| pb.library_order.contains(id)).collect();
        let after: Vec<ObjectId> = pb.library_order.iter().copied().filter(|id| common.contains(id)).collect();
        if common != after {
            v.push(format!("p{}'s library was reordered without a shuffle (CR 701.20a)", p.0));
        }
        // CR 121.3/701.13a: a draw and a mill take the top card. Order alone
        // cannot see the difference — taking from the bottom preserves the
        // order of what is left — so the cards that left have to be the ones
        // that were on top. A card put back into the library moves the top,
        // so a buffer containing one says nothing here.
        let put_back = events.iter().any(|e| matches!(e, GameEvent::ObjectMoved { object, to: Zone::Library, .. }
            if cur.get_object(*object).is_some_and(|o| o.owner == p)));
        if put_back {
            continue;
        }
        let taken: Vec<ObjectId> = events.iter().filter_map(|e| match e {
            GameEvent::ObjectMoved { object, from: Zone::Library, .. }
                if cur.get_object(*object).is_some_and(|o| o.owner == p) => Some(*object),
            _ => None,
        }).collect();
        // CR 121.3: a draw takes the top card. Only a draw — a card may say
        // it mills from the bottom (Cellar Door does), and an effect that
        // looks at the top few and sorts them removes them in its own order,
        // so the claim is about the drawn cards alone: they came from the
        // top of what was there, however many cards left in total.
        let drawn: BTreeSet<ObjectId> = events.iter().filter_map(|e| match e {
            GameEvent::CardDrawn { player, object } if *player == p => Some(*object),
            _ => None,
        }).collect();
        if !drawn.is_empty() {
            let top: BTreeSet<ObjectId> = pa.library_order.iter().take(taken.len().max(drawn.len())).copied().collect();
            if !drawn.is_subset(&top) {
                let below: Vec<ObjectId> = drawn.difference(&top).copied().collect();
                v.push(format!("p{} drew {below:?} from below the top {} of a library that starts {top:?} (CR 121.3)",
                    p.0, taken.len().max(drawn.len())));
            }
        }
    }
    // CR 121.1: drawn cards come out of the library the player had.
    for e in events {
        if let GameEvent::CardDrawn { player, object } = e {
            if !player_ok(prev, *player) {
                continue;
            }
            let was_there = prev.get_player(*player).library_order.contains(object);
            let refilled = match (prev.get_object(*object), cur.get_object(*object)) {
                (Some(a), Some(b)) => b.zone_change_count >= a.zone_change_count + 2,
                _ => false,
            };
            if !was_there && !refilled {
                v.push(format!("CardDrawn #{} which was not in p{}'s library (CR 121.1)", object.0, player.0));
            }
        }
    }
}

/// CR 119, 104.3, 704.5a/b, 121.4: life moves only through its events and a
/// loss says why.
fn life_and_loss(prev: &GameState, cur: &GameState, action: Option<&Action>, events: &[GameEvent], v: &mut Violations) {
    for (pa, pb) in prev.players.iter().zip(&cur.players) {
        let p = pa.id;
        let chain: Vec<(i32, i32)> = events.iter().filter_map(|e| match e {
            GameEvent::LifeChanged { player, old, new_life } if *player == p => Some((*old, *new_life)),
            _ => None,
        }).collect();
        match (chain.first(), chain.last()) {
            (None, _) => {
                if pb.life != pa.life {
                    v.push(format!("p{} life {} -> {} with no LifeChanged (CR 119)", p.0, pa.life, pb.life));
                }
            }
            (Some(first), Some(last)) => {
                if first.0 != pa.life {
                    v.push(format!("p{}'s life chain starts at {} but they had {}", p.0, first.0, pa.life));
                }
                if last.1 != pb.life {
                    v.push(format!("p{}'s life chain ends at {} but they have {}", p.0, last.1, pb.life));
                }
            }
            _ => {}
        }
        if !pa.lost && pb.lost {
            let announced = events.iter().any(|e| matches!(e, GameEvent::PlayerLost { player, reason }
                if *player == p && Some(*reason) == pb.loss_reason));
            if !announced {
                v.push(format!("p{} lost ({:?}) with no PlayerLost event", p.0, pb.loss_reason));
            }
            match pb.loss_reason {
                Some(LossReason::LifeReachedZero) => {
                    if pb.life > 0 || (pa.life > 0 && !chain.iter().any(|(_, n)| *n <= 0)) {
                        v.push(format!("p{} lost to 0 life without their life reaching 0 (CR 704.5a)", p.0));
                    }
                }
                Some(LossReason::DrewFromEmptyLibrary) => {
                    if !pb.has_drawn_from_empty {
                        v.push(format!("p{} lost to an empty-library draw that is not recorded (CR 704.5b)", p.0));
                    }
                }
                Some(LossReason::Conceded) => {
                    if !matches!(action, Some(Action::Concede)) || prev.priority_player != Some(p) {
                        v.push(format!("p{} conceded without holding priority on a Concede action", p.0));
                    }
                }
                Some(LossReason::OpponentWon) => {
                    if cur.result != Some(crate::state::GameResult::Winner(cur.opponent(p))) {
                        v.push(format!("p{} lost because the opponent won, but the result is {:?}", p.0, cur.result));
                    }
                }
                None => {}
            }
        }
        // CR 121.4: a failed draw is from an empty library.
        if !pa.has_drawn_from_empty && pb.has_drawn_from_empty && !pb.library_order.is_empty() {
            let refilled = cur.objects_in_id_order().iter().any(|b| b.zone == Zone::Library && b.owner == p
                && prev.get_object(b.id).is_none_or(|a| a.zone != Zone::Library || b.zone_change_count > a.zone_change_count));
            if !refilled {
                v.push(format!("p{} is recorded as drawing from an empty library that holds {} cards (CR 121.4)", p.0, pb.library_order.len()));
            }
        }
    }
}

/// CR 106.4, 500.4: mana appears only through ManaAdded and leaves only by
/// payment or the end of a step.
fn mana_ledger(prev: &GameState, cur: &GameState, action: Option<&Action>, events: &[GameEvent], v: &mut Violations) {
    let cleanup = events.iter().any(|e| matches!(e, GameEvent::StepStarted { step: Step::Cleanup }));
    // Nothing that resolves spends mana without asking first, so a pass
    // never pays.
    let paying = matches!(action, Some(Action::CastSpell { .. } | Action::ActivateAbility { .. }
        | Action::ActivateManaAbility { .. } | Action::ResolveChoice { .. }));
    for (pa, pb) in prev.players.iter().zip(&cur.players) {
        let p = pa.id;
        let emptied = events.iter().any(|e| matches!(e, GameEvent::ManaPoolEmptied { player } if *player == p));
        let types: BTreeSet<ManaType> = pa.mana_pool.mana.keys().chain(pb.mana_pool.mana.keys()).copied().collect();
        for t in types {
            let before = pa.mana_pool.mana.get(&t).copied().unwrap_or(0);
            let after = pb.mana_pool.mana.get(&t).copied().unwrap_or(0);
            let added: u32 = events.iter().map(|e| match e {
                GameEvent::ManaAdded { player, mana_type, amount } if *player == p && *mana_type == t => *amount,
                _ => 0,
            }).sum();
            if after > before + added {
                v.push(format!("p{} has {after} {t:?} mana after {before} + {added} added (CR 106.4)", p.0));
            }
            if after < before && !emptied && !cleanup && !paying {
                v.push(format!("p{} lost {t:?} mana ({before} -> {after}) with nothing paid and no step ending (CR 500.4)", p.0));
            }
        }
    }
}

/// Every mana symbol a cost demands, generic and colorless included. The
/// per-colour ledger below sees only the coloured pips, so a cost whose
/// generic part is never deducted reads as fully paid.
fn mana_demanded(cost: &crate::types::ManaCost) -> u32 {
    let colored = cost.symbols.iter().filter(|s| matches!(s, crate::types::ManaSymbol::Colored(_))).count();
    u32::try_from(colored).unwrap_or(u32::MAX) + cost.colorless_amount() + cost.generic_amount()
}

fn mana_added(events: &[GameEvent], who: PlayerId) -> u32 {
    events.iter().map(|e| match e {
        GameEvent::ManaAdded { player, amount, .. } if *player == who => *amount,
        _ => 0,
    }).sum()
}

fn entry_sig(e: &StackEntry) -> String {
    match e {
        StackEntry::Spell(id) => format!("spell #{}", id.0),
        StackEntry::Ability { source_id, ability_index, activator, .. } => format!("ability #{}/{} by p{}", source_id.0, ability_index, activator.0),
        StackEntry::Trigger(t) => format!("trigger #{} {}", t.source.id.0, t.source.description),
    }
}

/// What the chosen action must have done (CR 117.3c/117.4, 305, 601, 602,
/// 508.1, 509.1, 514.1, 103.4).
fn action_contract(prev: &GameState, cur: &GameState, action: &Action, events: &[GameEvent],
                   stayed: &dyn Fn(ObjectId) -> bool, registry: &CardRegistry, v: &mut Violations) {
    let p = prev.priority_player;
    let non_trigger = |s: &GameState| -> Vec<String> {
        s.stack.iter().filter(|e| !matches!(e, StackEntry::Trigger(_))).map(entry_sig).collect()
    };
    let zcc = |s: &GameState, id: ObjectId| s.get_object(id).map_or(0, |o| o.zone_change_count);
    let hand = |s: &GameState, who: PlayerId| s.objects_in_zone(Zone::Hand, who).len();

    // CR 405.2: a non-pass action grows the stack only from the top.
    if !matches!(action, Action::PassPriority | Action::ResolveChoice { .. }) {
        let before: Vec<String> = prev.stack.iter().map(entry_sig).collect();
        let after: Vec<String> = cur.stack.iter().map(entry_sig).collect();
        if after.len() < before.len() || after[..before.len()] != before[..] {
            v.push(format!("{action:?} disturbed the stack below the top: {before:?} -> {after:?} (CR 405.2)"));
        } else {
            let extra = &cur.stack[before.len()..];
            let own = extra.iter().filter(|e| !matches!(e, StackEntry::Trigger(_))).count();
            let allowed = matches!(action, Action::CastSpell { .. } | Action::ActivateAbility { .. } | Action::ActivateLoyaltyAbility { .. });
            if own > 1 || (own == 1 && !allowed) || (own == 1 && matches!(extra[0], StackEntry::Trigger(_))) {
                v.push(format!("{action:?} put {own} non-trigger entries on the stack: {:?}", extra.iter().map(entry_sig).collect::<Vec<_>>()));
            }
        }
    }

    let mut acted = false;
    match action {
        Action::PlayLand { object_id } => {
            acted = true;
            if prev.get_object(*object_id).is_none_or(|o| o.zone != Zone::Hand)
                || cur.get_object(*object_id).is_none_or(|o| o.zone != Zone::Battlefield)
                || !events.iter().any(|e| matches!(e, GameEvent::LandPlayed { object, player } if *object == *object_id && Some(*player) == p))
            {
                v.push(format!("PlayLand #{} did not put the land from hand onto the battlefield with its event (CR 305.1)", object_id.0));
            }
        }
        Action::CastSpell { object_id, alternative_cost, .. } => {
            let cast = events.iter().any(|e| matches!(e, GameEvent::SpellCast { object, .. } if *object == *object_id));
            let stashed = cur.pending_spell_cast.as_ref().is_some_and(|c| c.object_id == *object_id);
            if cast {
                acted = true;
                if zcc(cur, *object_id) <= zcc(prev, *object_id) {
                    v.push(format!("CastSpell #{} announced but the card never moved (CR 601.2a)", object_id.0));
                }
                // CR 601.2h: the colored part of the cost left the pool (no
                // hybrid mana in this pool, and reductions touch generic).
                if let (Some(who), Some(card)) = (p, prev.get_object(*object_id).map(|o| o.card_id)) {
                    // What the engine charges, not what the card prints: a
                    // reduction (Ghoultree and friends) is folded in here the
                    // same way the offer side folds it in.
                    let method = match alternative_cost {
                        Some(alt) => crate::engine::CastMethod::Alternative(alt.clone()),
                        None => crate::engine::CastMethod::Normal,
                    };
                    let cost = Some(crate::engine::cost_to_cast(prev, registry, card, who, &method).mana);
                    if let Some(cost) = cost {
                        // CR 601.2h: all of it, not just the colored part. X
                        // is announced and funded through its own prompt, so
                        // its size is not known from the state before the cast.
                        if !cost.has_x() {
                            let need_total = mana_demanded(&cost);
                            let before = prev.get_player(who).mana_pool.total();
                            let after = cur.get_player(who).mana_pool.total();
                            let added = mana_added(events, who);
                            if after + need_total > before + added {
                                v.push(format!("CastSpell #{} left p{} with {after} mana after {before} + {added} for a total cost of {need_total} (CR 601.2h)",
                                    object_id.0, who.0));
                            }
                        }
                        let mut need: HashMap<ManaType, u32> = HashMap::new();
                        for sym in &cost.symbols {
                            if let crate::types::ManaSymbol::Colored(c) = sym {
                                *need.entry(ManaType::from(*c)).or_default() += 1;
                            }
                        }
                        for (t, n) in need {
                            let before = prev.get_player(who).mana_pool.mana.get(&t).copied().unwrap_or(0);
                            let after = cur.get_player(who).mana_pool.mana.get(&t).copied().unwrap_or(0);
                            let added: u32 = events.iter().map(|e| match e {
                                GameEvent::ManaAdded { player, mana_type, amount } if *player == who && *mana_type == t => *amount,
                                _ => 0,
                            }).sum();
                            if after + n > before + added {
                                v.push(format!("CastSpell #{} left p{} with {after} {t:?} mana after {before} + {added} for a cost of {n} (CR 601.2h)",
                                    object_id.0, who.0));
                            }
                        }
                    }
                }
            } else if stashed {
                if !stayed(*object_id) {
                    v.push(format!("CastSpell #{} is waiting on a cost but the card moved", object_id.0));
                }
            } else if !stayed(*object_id) || !events.is_empty() || non_trigger(cur) != non_trigger(prev) {
                v.push(format!("CastSpell #{} was refused but left traces: {} events, stack {:?}", object_id.0, events.len(), non_trigger(cur)));
            }
        }
        Action::ActivateAbility { object_id, ability_index, source_card_id, .. } => {
            let pushed = cur.stack.iter().skip(prev.stack.len()).any(|e| matches!(e,
                StackEntry::Ability { source_id, ability_index: i, activator, .. }
                if *source_id == *object_id && *i == *ability_index && Some(*activator) == p));
            let stashed = cur.pending_ability_effect.as_ref().is_some_and(|a|
                a.source_id == *object_id && a.ability_index == *ability_index && Some(a.activator) == p);
            if pushed || stashed {
                // CR 602.2f: an activation cost is paid like any other, and
                // no ledger here ever looked at one.
                if let (Some(who), Some(o)) = (p, prev.get_object(*object_id)) {
                    let def_card = source_card_id.unwrap_or(o.card_id);
                    let def = registry.get(def_card)
                        .map(|b| b.activated_abilities(prev, *object_id, registry))
                        .and_then(|defs| defs.into_iter().find(|d| d.ability_index == *ability_index));
                    if let Some(def) = def.filter(|d| !d.cost.has_x()) {
                        let need = mana_demanded(&def.cost);
                        let before = prev.get_player(who).mana_pool.total();
                        let after = cur.get_player(who).mana_pool.total();
                        let added = mana_added(events, who);
                        if after + need > before + added {
                            v.push(format!("ActivateAbility #{}/{} left p{} with {after} mana after {before} + {added} for a cost of {need} (CR 602.2f)",
                                object_id.0, ability_index, who.0));
                        }
                    }
                }
            }
            if pushed {
                acted = true;
            } else if !stashed {
                let only_costs = events.iter().all(|e| matches!(e, GameEvent::ManaAdded { player, .. } if Some(*player) == p)
                    || matches!(e, GameEvent::Tapped { .. }));
                if non_trigger(cur) != non_trigger(prev) || !only_costs {
                    v.push(format!("ActivateAbility #{}/{} neither went on the stack nor was refused cleanly ({} events)", object_id.0, ability_index, events.len()));
                }
            }
        }
        Action::ActivateManaAbility { object_id, .. } => {
            if cur.step != prev.step || cur.priority_player != prev.priority_player || non_trigger(cur) != non_trigger(prev) {
                v.push(format!("mana ability of #{} changed the step, priority, or the stack (CR 605.3)", object_id.0));
            }
            if events.iter().any(|e| matches!(e, GameEvent::Tapped { object } if *object == *object_id))
                && prev.get_object(*object_id).is_some_and(|o| o.tapped)
            {
                v.push(format!("mana ability tapped #{} which was already tapped", object_id.0));
            }
        }
        Action::ActivateLoyaltyAbility { object_id, ability_index, .. } => {
            acted = true;
            let pushed = cur.stack.iter().skip(prev.stack.len()).any(|e| matches!(e,
                StackEntry::Ability { source_id, ability_index: i, .. } if *source_id == *object_id && *i == *ability_index));
            if !pushed {
                v.push(format!("loyalty ability #{}/{} did not go on the stack (CR 606.5)", object_id.0, ability_index));
            }
        }
        Action::DeclareAttackers { attackers, planeswalker_attacks } => {
            acted = true;
            let legal = crate::engine::legal_actions(prev, registry);
            let (eligible, must): (Vec<ObjectId>, Vec<ObjectId>) = match legal.combat_prompt {
                Some(CombatPrompt::ChooseAttackers { eligible, must_attack, .. }) => (eligible, must_attack),
                _ => {
                    v.push("DeclareAttackers submitted with no attackers prompt".into());
                    (vec![], vec![])
                }
            };
            let submitted: BTreeSet<ObjectId> = attackers.iter().map(|(id, _)| *id)
                .chain(planeswalker_attacks.iter().map(|(id, _)| *id)).collect();
            let Some(GameEvent::AttackersDeclared { attackers: declared }) =
                events.iter().find(|e| matches!(e, GameEvent::AttackersDeclared { .. })) else {
                v.push("DeclareAttackers without an AttackersDeclared event (CR 508.1)".into());
                return;
            };
            for (id, _) in declared {
                if !submitted.contains(id) && !must.contains(id) {
                    v.push(format!("#{} was declared attacking but was neither submitted nor forced", id.0));
                }
                if !eligible.contains(id) && !must.contains(id) {
                    v.push(format!("#{} was declared attacking but was not eligible (CR 508.1c)", id.0));
                }
            }
            if let Some(c) = &cur.combat {
                for id in c.attackers.keys() {
                    if !declared.iter().any(|(d, _)| d == id) && prev.get_object(*id).is_some() {
                        v.push(format!("#{} is attacking without having been declared", id.0));
                    }
                }
            }
            if matches!(cur.awaiting_action, Some(AwaitingAction::DeclareAttackers)) {
                v.push("the attackers prompt is still up after the declaration".into());
            }
        }
        Action::DeclareBlockers { assignments } => {
            acted = true;
            let Some(GameEvent::BlockersDeclared { assignments: declared }) =
                events.iter().find(|e| matches!(e, GameEvent::BlockersDeclared { .. })) else {
                v.push("DeclareBlockers without a BlockersDeclared event (CR 509.1)".into());
                return;
            };
            let defender = prev.opponent(prev.active_player);
            for (b, a) in declared {
                if !assignments.contains(&(*b, *a)) {
                    v.push(format!("#{} blocking #{} was declared but never submitted", b.0, a.0));
                }
                let ok = prev.get_object(*b).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == defender && !o.tapped)
                    && prev.is_creature(*b, registry)
                    && prev.combat.as_ref().is_some_and(|c| c.attackers.contains_key(a));
                if !ok {
                    v.push(format!("#{} blocking #{} was not a legal block (CR 509.1a)", b.0, a.0));
                }
            }
        }
        Action::DiscardCards { cards } => {
            if let Some(AwaitingAction::DiscardToHandSize { player, .. }) = &prev.awaiting_action {
                let who = *player;
                let discarded: Vec<ObjectId> = events.iter().filter_map(|e| match e {
                    GameEvent::Discarded { player, object } if *player == who => Some(*object),
                    _ => None,
                }).collect();
                if discarded != *cards {
                    v.push(format!("DiscardCards {cards:?} but p{} discarded {discarded:?} (CR 514.1)", who.0));
                }
                for c in cards {
                    if prev.get_object(*c).is_none_or(|o| o.zone != Zone::Hand) || stayed(*c) {
                        v.push(format!("discarded #{} was not moved out of p{}'s hand", c.0, who.0));
                    }
                }
                if hand(cur, who) + cards.len() != hand(prev, who) {
                    v.push(format!("p{}'s hand went {} -> {} discarding {} cards", who.0, hand(prev, who), hand(cur, who), cards.len()));
                }
            }
        }
        Action::MulliganMull => {
            acted = true;
            if let Some(AwaitingAction::MulliganDecision { player }) = &prev.awaiting_action {
                let who = *player;
                // CR 103.5: shuffle, then draw.
                let shuffled = events.iter().position(|e| matches!(e, GameEvent::LibraryShuffled { player } if *player == who));
                let first_draw = events.iter().position(|e| matches!(e, GameEvent::CardDrawn { player, .. } if *player == who));
                match (shuffled, first_draw) {
                    (None, _) => v.push(format!("p{} mulliganed without shuffling (CR 103.5)", who.0)),
                    (Some(s), Some(d)) if s > d => v.push(format!("p{} drew the new hand before shuffling (CR 103.5)", who.0)),
                    _ => {}
                }
                if cur.get_player(who).mulligan_count != prev.get_player(who).mulligan_count + 1 {
                    v.push(format!("p{} mulliganed without the count moving (CR 103.5)", who.0));
                }
                for o in prev.objects_in_zone(Zone::Hand, who) {
                    if stayed(o.id) {
                        v.push(format!("p{} mulliganed but #{} stayed in hand", who.0, o.id.0));
                    }
                }
                let drawn = events.iter().filter(|e| matches!(e, GameEvent::CardDrawn { player, .. } if *player == who)).count();
                if drawn != hand(cur, who) {
                    v.push(format!("p{} drew {drawn} cards for a new hand of {}", who.0, hand(cur, who)));
                }
            }
        }
        Action::MulliganKeep => {
            acted = true;
            if let Some(AwaitingAction::MulliganDecision { player }) = &prev.awaiting_action {
                if !cur.get_player(*player).mulligan_kept {
                    v.push(format!("p{} kept but is not recorded as having kept", player.0));
                }
            }
            if events.iter().any(|e| matches!(e, GameEvent::CardDrawn { .. })) {
                v.push("keeping a hand drew cards".into());
            }
        }
        Action::BottomCards { cards } => {
            acted = true;
            if let Some(AwaitingAction::BottomAfterMulligan { player, count }) = &prev.awaiting_action {
                let who = *player;
                if cards.len() != *count {
                    v.push(format!("p{} bottomed {} cards, asked for {count} (CR 103.5)", who.0, cards.len()));
                }
                for c in cards {
                    if prev.get_object(*c).is_none_or(|o| o.zone != Zone::Hand) || cur.get_object(*c).is_none_or(|o| o.zone != Zone::Library) {
                        v.push(format!("bottomed #{} did not go from hand to library", c.0));
                    }
                }
                let lib = &cur.get_player(who).library_order;
                let k = cards.len().min(lib.len());
                let tail: BTreeSet<ObjectId> = lib[lib.len() - k..].iter().copied().collect();
                let set: BTreeSet<ObjectId> = cards.iter().copied().collect();
                if lib.len() != prev.get_player(who).library_order.len() + cards.len() || tail != set {
                    v.push(format!("bottomed cards {cards:?} are not the bottom of p{}'s library", who.0));
                }
            }
        }
        Action::Concede => {
            if let Some(who) = p {
                if !cur.get_player(who).lost || cur.get_player(who).loss_reason != Some(LossReason::Conceded) {
                    v.push(format!("p{} conceded but is not recorded as having lost that way", who.0));
                }
            }
        }
        Action::PassPriority => {
            pass_contract(prev, cur, events, v);
        }
        Action::ResolveChoice { choice } => {
            // CR 704.5j: the legend kept stays; the rest are put into the graveyard.
            if let (Some(AwaitingAction::ResolutionChoice { player: who, choice: ResolutionChoiceKind::ChooseTarget {
                        options, effect: PendingEffect::LegendRuleKeep { .. }, .. }, .. }),
                    ResolvedChoice::ChosenTarget(Some(Target::Object(keep)))) = (&prev.awaiting_action, choice)
            {
                let doomed = prev.get_object(*keep).is_some_and(|o| o.damage_marked > 0 || o.dealt_deathtouch_damage
                    || prev.effective_toughness(*keep, registry).is_none_or(|t| t <= 0))
                    || events.iter().any(|e| matches!(e, GameEvent::LeftBattlefield { object, .. } if *object == *keep));
                // Only that it stays on the battlefield: the loser can have
                // been the source of a control effect over the winner (one
                // Olivia Voldaren stealing another), so the kept permanent
                // legitimately goes back to its owner as the loser dies.
                let _ = who;
                if !doomed && cur.get_object(*keep).is_none_or(|o| o.zone != Zone::Battlefield) {
                    v.push(format!("legend rule: the kept #{} did not stay on the battlefield (CR 704.5j)", keep.0));
                }
                for t in options {
                    if let Target::Object(id) = t {
                        if *id != *keep && cur.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield) {
                            v.push(format!("legend rule: #{} was not kept but is still on the battlefield (CR 704.5j)", id.0));
                        }
                    }
                }
            }
            // CR 608.2m, 601.2: a paused resolution or cast is resumed, not dropped.
            if let Some(id) = prev.resolving_spell {
                if cur.resolving_spell != Some(id) && cur.get_object(id).is_some_and(|o| o.zone == Zone::Stack) && stayed(id) {
                    v.push(format!("resolving spell #{} was dropped while still in the stack zone (CR 608.2m)", id.0));
                }
            }
            if let Some(c) = &prev.pending_spell_cast {
                let id = c.object_id;
                match &cur.pending_spell_cast {
                    Some(c2) if c2.object_id != id => v.push(format!("the pending cast switched from #{} to #{}", id.0, c2.object_id.0)),
                    None => {
                        let cast = events.iter().any(|e| matches!(e, GameEvent::SpellCast { object, .. } if *object == id));
                        if !cast && !stayed(id) {
                            v.push(format!("pending cast of #{} ended with the card moved but no SpellCast", id.0));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // CR 117.3c: a player who acts keeps priority; CR 117.4: acting resets
    // the succession of passes.
    if cur.result.is_none() {
        if matches!(action, Action::CastSpell { .. } | Action::ActivateAbility { .. } | Action::ActivateManaAbility { .. }
            | Action::ActivateLoyaltyAbility { .. } | Action::PlayLand { .. })
            && cur.priority_player != prev.priority_player
        {
            v.push(format!("{} handed priority {:?} -> {:?} (CR 117.3c)", action_name(action), prev.priority_player, cur.priority_player));
        }
        if acted && cur.consecutive_passes != 0 {
            v.push(format!("{} left {} consecutive passes standing (CR 117.4)", action_name(action), cur.consecutive_passes));
        }
    }
}

fn action_name(a: &Action) -> &'static str {
    match a {
        Action::PassPriority => "PassPriority",
        Action::PlayLand { .. } => "PlayLand",
        Action::CastSpell { .. } => "CastSpell",
        Action::ActivateManaAbility { .. } => "ActivateManaAbility",
        Action::ActivateAbility { .. } => "ActivateAbility",
        Action::ActivateLoyaltyAbility { .. } => "ActivateLoyaltyAbility",
        Action::DeclareAttackers { .. } => "DeclareAttackers",
        Action::DeclareBlockers { .. } => "DeclareBlockers",
        Action::DiscardCards { .. } => "DiscardCards",
        Action::MulliganKeep => "MulliganKeep",
        Action::MulliganMull => "MulliganMull",
        Action::BottomCards { .. } => "BottomCards",
        Action::Concede => "Concede",
        Action::ResolveChoice { .. } => "ResolveChoice",
    }
}

/// A digest of everything a lone pass must leave alone.
fn quiet_digest(s: &GameState) -> String {
    let mut objs: Vec<String> = s.objects_in_id_order().iter().map(|o| {
        let mut counters: Vec<(String, u32)> = o.counters.iter().filter(|(_, n)| **n > 0).map(|(k, n)| (format!("{k:?}"), *n)).collect();
        counters.sort();
        format!("{}:{:?}/{}/{}/{}/{}/{}/{:?}/{:?}/{:?}/{}/{:?}/{:?}/{}", o.id.0, o.zone, o.zone_change_count, o.tapped, o.controller.0,
            o.summoning_sick, o.damage_marked, counters, o.attached_to, o.attached_to_player, o.is_transformed, o.power, o.toughness,
            o.regeneration_shields)
    }).collect();
    objs.sort();
    let players: Vec<String> = s.players.iter().map(|p| format!("{}/{:?}/{:?}/{}/{}/{}", p.life, p.mana_pool.mana, p.library_order,
        p.land_plays_remaining, p.lost, p.has_drawn_from_empty)).collect();
    let stack: Vec<String> = s.stack.iter().map(entry_sig).collect();
    let combat = s.combat.as_ref().map(|c| {
        let mut a: Vec<_> = c.attackers.iter().map(|(k, d)| (k.0, d.0)).collect();
        a.sort_unstable();
        let mut b: Vec<_> = c.blocker_assignments.iter().map(|(k, bs)| (k.0, bs.iter().map(|x| x.0).collect::<Vec<_>>())).collect();
        b.sort();
        let mut blocked: Vec<_> = c.blocked_attackers.iter().map(|x| x.0).collect();
        blocked.sort_unstable();
        format!("{a:?}/{b:?}/{blocked:?}")
    });
    format!("{objs:?}|{players:?}|{stack:?}|{:?}/{}/{}|{combat:?}|{}/{}", s.step, s.turn_number, s.active_player.0,
        s.until_end_of_turn.len(), s.control_effects.len())
}

/// CR 117.4: the first pass only moves priority; the second resolves the
/// top of the stack or ends the step.
fn pass_contract(prev: &GameState, cur: &GameState, events: &[GameEvent], v: &mut Violations) {
    if prev.result.is_some() || cur.result.is_some() {
        return;
    }
    let Some(passer) = prev.priority_player else { return };
    if prev.consecutive_passes == 0 && prev.awaiting_action.is_none() {
        let only_pass = matches!(events, [GameEvent::PriorityPassed { player }] if *player == passer);
        if !only_pass {
            v.push(format!("a lone pass by p{} produced {} event(s): {:?}", passer.0, events.len(),
                events.iter().map(|e| format!("{e:?}").chars().take(40).collect::<String>()).collect::<Vec<_>>()));
        }
        if cur.consecutive_passes != 1 || cur.priority_player != Some(prev.opponent(passer)) || cur.awaiting_action.is_some() {
            v.push(format!("a lone pass by p{} left passes={} priority={:?} prompt={}", passer.0, cur.consecutive_passes,
                cur.priority_player, cur.awaiting_action.is_some()));
        }
        if quiet_digest(prev) != quiet_digest(cur) {
            v.push(format!("a lone pass by p{} changed the game (CR 117.4)", passer.0));
        }
        return;
    }
    if prev.consecutive_passes == 1 && prev.awaiting_action.is_none() {
        if cur.consecutive_passes != 0 {
            v.push(format!("passes stand at {} after everyone passed", cur.consecutive_passes));
        }
        match prev.stack.last() {
            None => {
                let started: Vec<Step> = events.iter().filter_map(|e| match e { GameEvent::StepStarted { step } => Some(*step), _ => None }).collect();
                if started.is_empty() {
                    v.push("everyone passed on an empty stack but no step started (CR 117.4)".into());
                } else if started.last() != Some(&cur.step) {
                    v.push(format!("the step walk ended at {:?} but the step is {:?}", started.last(), cur.step));
                }
            }
            Some(top) => {
                let k = prev.stack.len() - 1;
                let below: Vec<String> = prev.stack[..k].iter().map(entry_sig).collect();
                let now: Vec<String> = cur.stack.iter().filter(|e| !matches!(e, StackEntry::Trigger(_))).map(entry_sig).collect();
                // Resolution may counter or remove entries below it, never
                // add non-trigger ones or reorder the survivors.
                let mut it = below.iter();
                let subseq = now.iter().all(|n| it.any(|b| b == n));
                if !subseq {
                    v.push(format!("resolving {} left the stack {now:?}, not a subsequence of {below:?} (CR 608.2n)", entry_sig(top)));
                }
                let sig = entry_sig(top);
                let before = prev.stack.iter().filter(|e| entry_sig(e) == sig).count();
                let after = cur.stack.iter().filter(|e| entry_sig(e) == sig).count();
                if after >= before && !matches!(top, StackEntry::Trigger(_)) {
                    v.push(format!("{sig} is still on the stack after resolving"));
                }
                match top {
                    StackEntry::Spell(id) => {
                        let in_stack_zone = cur.get_object(*id).is_some_and(|o| o.zone == Zone::Stack);
                        if in_stack_zone && cur.resolving_spell != Some(*id) {
                            v.push(format!("spell #{} resolved but is still in the stack zone with no resolution in progress", id.0));
                        }
                        let resolved = events.iter().any(|e| matches!(e, GameEvent::SpellResolved { object } if *object == *id));
                        if !resolved && in_stack_zone && cur.resolving_spell != Some(*id) {
                            v.push(format!("spell #{} left the top of the stack without resolving", id.0));
                        }
                    }
                    StackEntry::Ability { x_value, sacrificed, .. } => {
                        if cur.last_activated_x_value != *x_value || cur.last_activated_sacrifice != *sacrificed {
                            v.push(format!("ability resolved with X={:?}/sacrifice={:?} but the state says X={:?}/sacrifice={:?}",
                                x_value, sacrificed, cur.last_activated_x_value, cur.last_activated_sacrifice));
                        }
                    }
                    StackEntry::Trigger(_) => {}
                }
                if cur.awaiting_action.is_none() && cur.priority_player != Some(cur.active_player) {
                    v.push(format!("after a resolution priority is {:?}, not the active player's (CR 117.3b)", cur.priority_player));
                }
            }
        }
    }
}

fn trigger_sig(t: &PendingTrigger) -> (ObjectId, String) {
    (t.source.id, t.source.description.clone())
}

/// CR 603.2: every trigger that appeared this transition has its event in
/// the buffer.
fn triggers_witnessed(prev: &GameState, cur: &GameState, events: &[GameEvent], v: &mut Violations) {
    if prev.awaiting_action.is_some() || cur.result.is_some() {
        return;
    }
    let mut old: HashMap<(ObjectId, String), usize> = HashMap::new();
    for t in prev.stack.iter().filter_map(StackEntry::as_trigger)
        .chain(prev.pending_trigger_pushes_ap.iter()).chain(prev.pending_trigger_pushes_nap.iter()).chain(prev.pending_triggers.iter())
    {
        *old.entry(trigger_sig(t)).or_default() += 1;
    }
    let new_triggers = cur.stack.iter().filter_map(StackEntry::as_trigger)
        .chain(cur.pending_trigger_pushes_ap.iter()).chain(cur.pending_trigger_pushes_nap.iter()).chain(cur.pending_triggers.iter());
    for t in new_triggers {
        if let Some(n) = old.get_mut(&trigger_sig(t)) {
            if *n > 0 {
                *n -= 1;
                continue;
            }
        }
        let src = t.source.id;
        let has = |f: &dyn Fn(&GameEvent) -> bool| events.iter().any(f);
        let witnessed = match &t.event {
            TriggerEvent::SelfEntered => cur.get_object(src).is_some_and(|o| o.copy_grantor.is_some())
                || has(&|e| matches!(e, GameEvent::EnteredBattlefield { object, .. } if *object == src)),
            TriggerEvent::SelfDies => has(&|e| matches!(e, GameEvent::CreatureDied { object, .. } if *object == src)),
            TriggerEvent::CreatureDied { dead } => has(&|e| matches!(e, GameEvent::CreatureDied { object, .. } if *object == dead.id)),
            TriggerEvent::CreatureEntered { entered, .. } => has(&|e| matches!(e, GameEvent::EnteredBattlefield { object, .. } if *object == *entered)),
            TriggerEvent::Attacks { attacker, .. } | TriggerEvent::CreatureAttacked { attacker, .. } =>
                has(&|e| matches!(e, GameEvent::AttackersDeclared { attackers } if attackers.iter().any(|(a, _)| *a == *attacker))),
            TriggerEvent::Blocks { .. } | TriggerEvent::BecomesBlocked { .. } => has(&|e| matches!(e, GameEvent::BlockersDeclared { .. })),
            TriggerEvent::CombatDamageToPlayer { .. } => has(&|e| matches!(e, GameEvent::CombatDamageDealt { target: DamageTarget::Player(_), .. })),
            TriggerEvent::AnyCombatDamageToPlayer { dealer, .. } =>
                has(&|e| matches!(e, GameEvent::CombatDamageDealt { source, target: DamageTarget::Player(_), .. } if *source == *dealer)),
            TriggerEvent::AnyDamageToPlayer { dealer, .. } => has(&|e| matches!(e,
                GameEvent::CombatDamageDealt { source, target: DamageTarget::Player(_), .. }
                | GameEvent::NonCombatDamageDealt { source, target: DamageTarget::Player(_), .. } if *source == *dealer)),
            TriggerEvent::CombatDamageToCreature { damaged_creature, .. } =>
                has(&|e| matches!(e, GameEvent::CombatDamageDealt { target: DamageTarget::Object(o), .. } if *o == *damaged_creature)),
            TriggerEvent::SpellCast { spell_id, .. } => has(&|e| matches!(e, GameEvent::SpellCast { object, .. } if *object == *spell_id)),
            TriggerEvent::CreatureCardMilled { milled_object, .. } =>
                has(&|e| matches!(e, GameEvent::CreatureCardMilled { object, .. } if *object == *milled_object)),
            TriggerEvent::LeftBattlefield => has(&|e| matches!(e, GameEvent::LeftBattlefield { object, .. } if *object == src)),
            TriggerEvent::Upkeep => cur.step == Step::Upkeep && has(&|e| matches!(e, GameEvent::StepStarted { step: Step::Upkeep })),
            TriggerEvent::EndStep => cur.step == Step::EndStep && has(&|e| matches!(e, GameEvent::StepStarted { step: Step::EndStep })),
            TriggerEvent::EndCombat => cur.step == Step::EndCombat && has(&|e| matches!(e, GameEvent::StepStarted { step: Step::EndCombat })),
            TriggerEvent::StateTriggered | TriggerEvent::DelayedTokenExile { .. } => true,
        };
        if !witnessed {
            v.push(format!("trigger {} of #{} ({:?}) appeared with no event to trigger it (CR 603.2)", t.source.description, src.0,
                std::mem::discriminant(&t.event)));
        }
    }
}

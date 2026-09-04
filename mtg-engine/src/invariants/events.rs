//! Event-window invariants: what the events of the current action say the
//! state must look like at the decision point that follows.
//!
//! `submit_action` clears `state.events`, and between an action and the
//! next decision point only state-based actions, trigger collection, and a
//! step advance can run (`engine::run_game_loop_inner`). So an event in the
//! buffer was produced just now, and — unless something left the
//! battlefield in between (`quiet`) — every characteristic read here equals
//! its value at the moment of the event. That is what makes declaration-
//! time rules (CR 508.1, 509.1) checkable after the fact. Core tier: the
//! events that matter are never produced inside a resolution.

use super::{player_ok, Violations};
use crate::cards::CardRegistry;
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, StackEntry};
use crate::types::{CardType, Keyword, Step, Zone};

/// Nothing left the battlefield in this action, so characteristics now are
/// characteristics then.
fn quiet(state: &GameState) -> bool {
    !state.events.iter().any(|e| matches!(e, GameEvent::LeftBattlefield { .. }))
}

fn on_bf(state: &GameState, id: ObjectId) -> bool {
    state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield)
}

pub(super) fn check_core(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    let events = &state.events;
    let quiet = quiet(state);
    let active = state.active_player;
    let def = state.opponent(active);

    // Every player named by an event is a player.
    for e in events {
        let p = match e {
            GameEvent::TurnStarted { player, .. } | GameEvent::CardDrawn { player, .. }
            | GameEvent::LandPlayed { player, .. } | GameEvent::SpellCast { player, .. }
            | GameEvent::ManaAdded { player, .. } | GameEvent::ManaPoolEmptied { player }
            | GameEvent::LifeChanged { player, .. } | GameEvent::PlayerLost { player, .. }
            | GameEvent::PriorityPassed { player } | GameEvent::Discarded { player, .. }
            | GameEvent::LibraryShuffled { player } => Some(*player),
            GameEvent::EnteredBattlefield { controller, .. } | GameEvent::CreatureDied { controller, .. } => Some(*controller),
            GameEvent::LeftBattlefield { last_controller, .. } => Some(*last_controller),
            GameEvent::CreatureCardMilled { milled_player, .. } => Some(*milled_player),
            GameEvent::CombatDamageDealt { target: DamageTarget::Player(p), .. }
            | GameEvent::NonCombatDamageDealt { target: DamageTarget::Player(p), .. } => Some(*p),
            _ => None,
        };
        if let Some(p) = p {
            if !player_ok(state, p) {
                v.push(format!("event {e:?} names p{} who is not a player", p.0));
            }
        }
    }

    spells_and_lands(state, registry, events, quiet, v);
    attackers(state, registry, events, quiet, v);
    blockers(state, registry, events, quiet, v);
    damage(state, registry, events, quiet, v);
    zone_changes(state, registry, events, v);
    steps(state, registry, events, v);

    let _ = (active, def);
}

fn spells_and_lands(state: &GameState, registry: &CardRegistry, events: &[GameEvent], quiet: bool, v: &mut Violations) {
    let mut cast_objects = std::collections::HashSet::new();
    let mut casts_by: std::collections::HashMap<PlayerId, u32> = std::collections::HashMap::new();
    let mut land_plays = 0;
    for e in events {
        match e {
            GameEvent::SpellCast { player, object } => {
                let what = format!("SpellCast of #{} by p{}", object.0, player.0);
                if !cast_objects.insert(*object) {
                    v.push(format!("{what} twice in one action (CR 601.2i)"));
                }
                *casts_by.entry(*player).or_default() += 1;
                match state.get_object(*object) {
                    None => v.push(format!("{what}: no such object")),
                    Some(o) => {
                        if o.zone != Zone::Stack || o.controller != *player {
                            v.push(format!("{what} but it is in {:?} under p{} (CR 112.1/112.2)", o.zone, o.controller.0));
                        }
                        if state.has_card_type(*object, CardType::Land, registry) {
                            v.push(format!("{what} but it is a land (CR 305.9)"));
                        }
                        if quiet && on_bf(state, *object) {
                            // Unreachable but keeps the shape symmetric.
                        }
                        // CR 702.11b/702.16b: chosen targets could be targeted.
                        if quiet {
                            for t in &o.targets {
                                match t {
                                    crate::actions::Target::Object(tid) if on_bf(state, *tid) => {
                                        if state.has_keyword(*tid, Keyword::Hexproof, registry)
                                            && state.get_object(*tid).is_some_and(|x| x.controller != *player)
                                        {
                                            v.push(format!("{what} targets #{} which has hexproof from p{} (CR 702.11b)", tid.0, player.0));
                                        }
                                        if state.has_protection_from(*tid, *object, registry) {
                                            v.push(format!("{what} targets #{} which has protection from it (CR 702.16b)", tid.0));
                                        }
                                    }
                                    crate::actions::Target::Player(p) if *p != *player
                                        && player_ok(state, *p) && state.player_has_hexproof(*p, registry) => {
                                        v.push(format!("{what} targets p{} who has hexproof (CR 702.11c)", p.0));
                                    }
                                    crate::actions::Target::Illegal => v.push(format!("{what} carries an Illegal target")),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if !state.stack.iter().any(|s| s.as_spell() == Some(*object)) {
                    v.push(format!("{what} but it is on no stack entry (CR 112.1)"));
                }
            }
            GameEvent::SpellResolved { object } => {
                if state.stack.iter().any(|s| s.as_spell() == Some(*object)) {
                    v.push(format!("SpellResolved for #{} which is still on the stack (CR 608.2n)", object.0));
                }
            }
            GameEvent::LandPlayed { player, object } => {
                land_plays += 1;
                let what = format!("LandPlayed #{} by p{}", object.0, player.0);
                if *player != state.active_player || !state.step.is_main_phase() {
                    v.push(format!("{what} on p{}'s turn in {:?} (CR 305.1)", state.active_player.0, state.step));
                }
                if player_ok(state, *player) && state.get_player(*player).land_plays_remaining != 0 {
                    v.push(format!("{what} but the land drop was not spent (CR 305.2)"));
                }
                match state.get_object(*object) {
                    Some(o) if o.zone == Zone::Battlefield && o.owner == *player && o.controller == *player
                        && state.has_card_type(*object, CardType::Land, registry) && !o.is_token => {
                        if o.attached_to.is_some() {
                            v.push(format!("{what} arrived attached"));
                        }
                    }
                    _ => v.push(format!("{what} but no such land of p{} on the battlefield", player.0)),
                }
                if !events.iter().any(|e| matches!(e, GameEvent::EnteredBattlefield { object: o, controller } if o == object && controller == player)) {
                    v.push(format!("{what} without its EnteredBattlefield"));
                }
                if state.stack.iter().any(|s| !matches!(s, StackEntry::Trigger(_))) || state.resolving_spell.is_some() {
                    v.push(format!("{what} with a spell or ability on the stack (CR 305.1)"));
                }
            }
            _ => {}
        }
    }
    if land_plays > 1 {
        v.push(format!("{land_plays} lands played in one action (CR 305.2)"));
    }
    for (p, n) in casts_by {
        if player_ok(state, p) && state.num_spells_cast_this_turn.get(&p).copied().unwrap_or(0) < n {
            v.push(format!("p{} cast {n} spell(s) this action but the turn's count says {}", p.0,
                state.num_spells_cast_this_turn.get(&p).copied().unwrap_or(0)));
        }
    }
    // CR 117.3c: the player who cast or played keeps priority afterwards.
    if state.awaiting_action.is_none() && state.result.is_none() {
        for e in events {
            let (who, what) = match e {
                GameEvent::SpellCast { player, .. } => (*player, "cast a spell"),
                GameEvent::LandPlayed { player, .. } => (*player, "played a land"),
                _ => continue,
            };
            if state.priority_player != Some(who) {
                v.push(format!("p{} {what} but priority is {:?} (CR 117.3c)", who.0, state.priority_player));
            }
        }
    }
}

fn attackers(state: &GameState, registry: &CardRegistry, events: &[GameEvent], quiet: bool, v: &mut Violations) {
    let Some(declared) = events.iter().find_map(|e| match e {
        GameEvent::AttackersDeclared { attackers } => Some(attackers),
        _ => None,
    }) else { return };
    let Some(c) = &state.combat else {
        v.push("AttackersDeclared but no combat state".into());
        return;
    };
    let def = state.opponent(state.active_player);
    let tapped_now: std::collections::HashSet<ObjectId> = events.iter().filter_map(|e| match e {
        GameEvent::Tapped { object } => Some(*object), _ => None }).collect();
    for (id, d) in declared {
        let what = format!("declared attacker #{}", id.0);
        if *d != def {
            v.push(format!("{what} attacks p{}, not the defending player (CR 506.2)", d.0));
        }
        let Some(o) = state.get_object(*id) else { continue };
        if o.zone != Zone::Battlefield {
            continue;
        }
        if !c.attackers.contains_key(id) {
            v.push(format!("{what} is not in combat"));
        }
        if o.attacked_on_turn != Some(state.turn_number) {
            v.push(format!("{what} is not stamped as attacking this turn (CR 508.1)"));
        }
        if o.controller != state.active_player {
            v.push(format!("{what} is controlled by p{} (CR 508.1a)", o.controller.0));
        }
        if !state.is_creature(*id, registry) {
            v.push(format!("{what} is not a creature (CR 506.3)"));
        }
        if quiet {
            if o.summoning_sick && !state.has_keyword(*id, Keyword::Haste, registry) {
                v.push(format!("{what} is summoning sick without haste (CR 302.6)"));
            }
            if state.has_keyword(*id, Keyword::Defender, registry) {
                v.push(format!("{what} has defender (CR 702.3b)"));
            }
            if !state.can_attack(*id, registry) {
                v.push(format!("{what} can't attack (CR 508.1c)"));
            }
            if !state.has_keyword(*id, Keyword::Vigilance, registry) && !(o.tapped && tapped_now.contains(id)) {
                v.push(format!("{what} was not tapped by attacking (CR 508.1f)"));
            }
        }
    }
    if quiet {
        for id in c.attackers.keys() {
            if !declared.iter().any(|(d, _)| d == id) {
                v.push(format!("#{} is attacking but was not declared (CR 508.1)", id.0));
            }
        }
    }
}

fn blockers(state: &GameState, registry: &CardRegistry, events: &[GameEvent], quiet: bool, v: &mut Violations) {
    let Some(assignments) = events.iter().find_map(|e| match e {
        GameEvent::BlockersDeclared { assignments } => Some(assignments),
        _ => None,
    }) else { return };
    let Some(c) = &state.combat else {
        v.push("BlockersDeclared but no combat state".into());
        return;
    };
    let def = state.opponent(state.active_player);
    let left: std::collections::HashSet<ObjectId> = events.iter().filter_map(|e| match e {
        GameEvent::LeftBattlefield { object, .. } => Some(*object), _ => None }).collect();
    let mut seen_blockers = std::collections::HashSet::new();
    let mut per_attacker: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
    for (b, a) in assignments {
        let what = format!("declared block #{} -> #{}", b.0, a.0);
        if !seen_blockers.insert(*b) {
            v.push(format!("{what}: blocker declared twice (CR 509.1b)"));
        }
        *per_attacker.entry(*a).or_default() += 1;
        if !left.contains(a) {
            if !c.attackers.contains_key(a) {
                v.push(format!("{what}: not an attacker"));
            }
            if !c.blocker_assignments.get(a).is_some_and(|bs| bs.contains(b)) || !c.blocked_attackers.contains(a) {
                v.push(format!("{what} is not recorded in combat (CR 509.1h)"));
            }
        }
        if left.contains(b) {
            continue;
        }
        let Some(bo) = state.get_object(*b) else { continue };
        if bo.zone != Zone::Battlefield || bo.controller != def || !state.is_creature(*b, registry) {
            v.push(format!("{what}: blocker is not a creature the defending player controls (CR 509.1a)"));
        }
        if bo.tapped {
            v.push(format!("{what}: blocker is tapped (CR 509.1a)"));
        }
        if quiet {
            if !state.can_block(*b, registry) {
                v.push(format!("{what}: blocker can't block"));
            }
            if on_bf(state, *a) {
                if state.has_keyword(*a, Keyword::Flying, registry)
                    && !state.has_keyword(*b, Keyword::Flying, registry)
                    && !state.has_keyword(*b, Keyword::Reach, registry)
                {
                    v.push(format!("{what}: a flier blocked by neither flying nor reach (CR 702.9b)"));
                }
                if state.has_keyword(*a, Keyword::Intimidate, registry)
                    && !state.has_card_type(*b, CardType::Artifact, registry)
                    && !state.colors_of(*a, registry).iter().any(|col| state.colors_of(*b, registry).contains(col))
                {
                    v.push(format!("{what}: intimidate blocked by a non-artifact sharing no color (CR 702.13b)"));
                }
                if state.cant_be_blocked(*a, registry) {
                    v.push(format!("{what}: the attacker can't be blocked"));
                }
                if state.has_protection_from(*a, *b, registry) {
                    v.push(format!("{what}: the attacker has protection from the blocker (CR 702.16f)"));
                }
            }
        }
    }
    if quiet {
        for (a, n) in &per_attacker {
            if on_bf(state, *a) && state.has_keyword(*a, Keyword::Menace, registry) && *n < 2 {
                v.push(format!("#{} has menace but was blocked by {n} creature (CR 702.111b)", a.0));
            }
        }
        for (a, bs) in &c.blocker_assignments {
            for b in bs {
                if !assignments.contains(&(*b, *a)) {
                    v.push(format!("block #{} -> #{} is in combat but was not declared", b.0, a.0));
                }
            }
        }
        for a in &c.blocked_attackers {
            if !assignments.iter().any(|(_, x)| x == a) {
                v.push(format!("#{} is marked blocked but no block was declared for it", a.0));
            }
        }
    }
}

fn damage(state: &GameState, registry: &CardRegistry, events: &[GameEvent], quiet: bool, v: &mut Violations) {
    // Life-change events consumed by damage/lifelink pairings, by index.
    let mut consumed = vec![false; events.len()];
    let combat_damage = events.iter().any(|e| matches!(e, GameEvent::CombatDamageDealt { .. }));
    if combat_damage && (state.step != Step::CombatDamage || state.combat.is_none()) {
        v.push(format!("combat damage dealt in {:?} (CR 510.2)", state.step));
    }
    let def = state.opponent(state.active_player);
    for (i, e) in events.iter().enumerate() {
        let (source, target, amount, combat) = match e {
            GameEvent::CombatDamageDealt { source, target, amount } => (*source, target, *amount, true),
            GameEvent::NonCombatDamageDealt { source, target, amount } => (*source, target, *amount, false),
            _ => continue,
        };
        let what = format!("{} damage {} from #{} to {:?}", if combat { "combat" } else { "noncombat" }, amount, source.0, target);
        // CR 120.8: zero damage is no damage.
        if amount == 0 {
            v.push(format!("{what}: a zero-damage event (CR 120.8)"));
        }
        match target {
            DamageTarget::Object(t) => {
                // CR 120.1a: damage lands on creatures and planeswalkers.
                if let Some(o) = state.get_object(*t) {
                    if !state.is_creature(*t, registry) && !state.has_card_type(*t, CardType::Planeswalker, registry) {
                        v.push(format!("{what}: {} is neither creature nor planeswalker (CR 120.1a)", o.name));
                    }
                    if quiet && o.zone == Zone::Battlefield && on_bf(state, source)
                        && state.has_protection_from(*t, source, registry)
                    {
                        v.push(format!("{what}: the target has protection from the source (CR 702.16e)"));
                    }
                }
            }
            DamageTarget::Player(p) => {
                // CR 120.3a: damage to a player is life loss, recorded first.
                let paired = events[..i].iter().enumerate().rev().find(|(j, x)| !consumed[*j] && matches!(x,
                    GameEvent::LifeChanged { player, old, new_life } if player == p && old - new_life == amount as i32));
                match paired {
                    Some((j, _)) => consumed[j] = true,
                    None => v.push(format!("{what}: no matching life loss for p{} (CR 120.3a)", p.0)),
                }
                if combat && *p != def {
                    v.push(format!("{what}: combat damage to p{} who is not the defending player (CR 506.2)", p.0));
                }
                if quiet && on_bf(state, source) && player_ok(state, *p) {
                    for col in state.colors_of(source, registry) {
                        if state.player_has_protection_from(*p, col, registry) {
                            v.push(format!("{what}: p{} has protection from {col:?} (CR 702.16e)", p.0));
                        }
                    }
                }
            }
        }
        // CR 702.15b: lifelink damage gains life for the source's controller.
        if on_bf(state, source) && state.has_keyword(source, Keyword::Lifelink, registry) {
            let controller = state.get_object(source).map(|o| o.controller);
            let gain = events.iter().enumerate().skip(i + 1).find(|(j, x)| !consumed[*j] && matches!(x,
                GameEvent::LifeChanged { player, old, new_life } if Some(*player) == controller && new_life - old == amount as i32));
            match gain {
                Some((j, _)) => consumed[j] = true,
                None => v.push(format!("{what}: lifelink but no life gain for its controller (CR 702.15b)")),
            }
        }
        // Combat assignment rules (CR 510.1).
        if combat {
            let Some(c) = &state.combat else { continue };
            if state.get_object(source).is_none()
                && !events.iter().any(|x| matches!(x, GameEvent::CreatureDied { object, .. } if *object == source))
            {
                v.push(format!("{what}: the source neither exists nor died"));
            }
            if c.attackers.contains_key(&source) {
                let blocked = c.blocked_attackers.contains(&source);
                let to_walker = matches!(target, DamageTarget::Object(w) if c.planeswalker_defenders.get(&source) == Some(w));
                match target {
                    DamageTarget::Player(_) | DamageTarget::Object(_) if blocked && (matches!(target, DamageTarget::Player(_)) || to_walker) => {
                        // Not gated on a quiet window: CR 509.2 keeps an
                        // attacker blocked for the rest of combat even when
                        // every blocker has left, which is exactly the
                        // window where the damage code is most likely to
                        // forget it.
                        if on_bf(state, source) && !state.has_keyword(source, Keyword::Trample, registry) {
                            v.push(format!("{what}: a blocked attacker without trample reached the player (CR 510.1c)"));
                        }
                    }
                    DamageTarget::Object(o) if blocked => {
                        let listed = c.blocker_assignments.get(&source).is_some_and(|bs| bs.contains(o));
                        let gone = !on_bf(state, *o) || state.get_object(*o).is_some_and(|x| x.damage_marked == 0);
                        if !listed && !gone && !(quiet && state.has_keyword(source, Keyword::Trample, registry) && to_walker) {
                            v.push(format!("{what}: a blocked attacker hit #{} which is not blocking it (CR 510.1c)", o.0));
                        }
                    }
                    DamageTarget::Player(p) => {
                        if !blocked && *p != def {
                            v.push(format!("{what}: unblocked attacker hit p{} (CR 510.1b)", p.0));
                        }
                    }
                    DamageTarget::Object(o) => {
                        if !blocked && !to_walker {
                            v.push(format!("{what}: unblocked attacker hit #{} which it is not attacking (CR 510.1b)", o.0));
                        }
                    }
                }
            } else if let Some((a, _)) = c.blocker_assignments.iter().find(|(_, bs)| bs.contains(&source)) {
                // A blocker's damage goes to the attacker it blocks (CR 510.1d).
                if !matches!(target, DamageTarget::Object(t) if t == a) && on_bf(state, *a) {
                    v.push(format!("{what}: a blocker of #{} hit something else (CR 510.1d)", a.0));
                }
            }
            // CR 510.4: first-strike step discipline.
            if quiet && on_bf(state, source) {
                let fs = state.has_keyword(source, Keyword::FirstStrike, registry)
                    || state.has_keyword(source, Keyword::DoubleStrike, registry);
                if state.combat_damage_step_pending && !fs {
                    v.push(format!("{what}: dealt in the first-strike step without first strike (CR 510.4)"));
                }
                if !state.combat_damage_step_pending && c.dealt_first_strike.contains(&source)
                    && !state.has_keyword(source, Keyword::DoubleStrike, registry)
                {
                    v.push(format!("{what}: dealt regular damage after first-strike damage without double strike (CR 702.4b)"));
                }
            }
        }
    }
}

/// Whether `e` is about object `id` — the events that can move or mark it.
fn names_object(e: &GameEvent, id: ObjectId) -> bool {
    match e {
        GameEvent::CardDrawn { object, .. } | GameEvent::LandPlayed { object, .. }
        | GameEvent::SpellCast { object, .. } | GameEvent::SpellResolved { object }
        | GameEvent::EnteredBattlefield { object, .. } | GameEvent::LeftBattlefield { object, .. }
        | GameEvent::ObjectMoved { object, .. } | GameEvent::Tapped { object } | GameEvent::Untapped { object }
        | GameEvent::CreatureDied { object, .. } | GameEvent::Discarded { object, .. }
        | GameEvent::CreatureCardMilled { object, .. } => *object == id,
        _ => false,
    }
}

/// CR 502.3: the untap step untaps the active player's permanents and
/// nothing else, and nothing else happens in it.
fn untap_step_scope(state: &GameState, events: &[GameEvent], v: &mut Violations) {
    let Some(i) = events.iter().rposition(|e| matches!(e, GameEvent::StepStarted { step: Step::Untap })) else { return };
    let j = events[i + 1..].iter().position(|e| matches!(e, GameEvent::StepStarted { .. })).map_or(events.len(), |k| i + 1 + k);
    for e in &events[i + 1..j] {
        if let GameEvent::Untapped { object } = e {
            let ok = state.get_object(*object)
                .is_none_or(|o| o.zone != Zone::Battlefield || o.controller == state.active_player);
            if !ok {
                v.push(format!("the untap step untapped #{} which p{} does not control (CR 502.3)", object.0, state.active_player.0));
            }
        }
    }
}

fn zone_changes(state: &GameState, registry: &CardRegistry, events: &[GameEvent], v: &mut Violations) {
    untap_step_scope(state, events, v);
    for (i, e) in events.iter().enumerate() {
        let later_mention = |id: ObjectId| events[i + 1..].iter().any(|x| names_object(x, id));
        match e {
            GameEvent::CardDrawn { player, object } => {
                let ok = state.get_object(*object).is_some_and(|o|
                    o.owner == *player && !o.is_token && o.zone != Zone::Library);
                if !ok {
                    v.push(format!("CardDrawn #{} by p{}: not that player's card out of their library (CR 121.1)", object.0, player.0));
                }
                if player_ok(state, *player) && state.get_player(*player).library_order.contains(object) {
                    v.push(format!("CardDrawn #{} is still listed in p{}'s library", object.0, player.0));
                }
                // CR 121.1: drawing puts the card in hand, and nothing in this
                // pool moves it again in the same action without saying so.
                if !later_mention(*object) && state.get_object(*object).is_some_and(|o| o.zone != Zone::Hand) {
                    v.push(format!("CardDrawn #{} but it is in {:?} (CR 121.1)", object.0, state.get_object(*object).map(|o| o.zone).unwrap()));
                }
            }
            GameEvent::Discarded { player, object } => {
                if !state.get_object(*object).is_some_and(|o| !o.is_token && o.owner == *player) {
                    v.push(format!("Discarded #{} by p{}: not that player's card (CR 701.9a)", object.0, player.0));
                }
                // CR 701.8a: discarding puts the card in the graveyard.
                if !later_mention(*object) && state.get_object(*object).is_some_and(|o| o.zone != Zone::Graveyard) {
                    v.push(format!("Discarded #{} but it is in {:?} (CR 701.8a)", object.0, state.get_object(*object).map(|o| o.zone).unwrap()));
                }
            }
            // CR 104.2a: a loss ends the game in the same breath.
            GameEvent::PlayerLost { player, .. } => {
                if !events[i + 1..].iter().any(|x| matches!(x, GameEvent::GameEnded { .. })) {
                    v.push(format!("PlayerLost p{} without the game ending afterwards (CR 104.2a)", player.0));
                }
            }

            GameEvent::CreatureCardMilled { object, milled_player } => {
                if !state.get_object(*object).is_some_and(|o| o.owner == *milled_player) {
                    v.push(format!("CreatureCardMilled #{} for p{}: not that player's card (CR 701.17a)", object.0, milled_player.0));
                }
            }
            // CR 700.4: dying is leaving the battlefield for the graveyard,
            // reported once with the controller it had.
            GameEvent::CreatureDied { object, controller, is_token, .. } => {
                let after = &events[i + 1..];
                // Morbid reads this flag; every death path sets it.
                if !state.creature_died_this_turn && !after.iter().any(|x| matches!(x, GameEvent::TurnStarted { .. })) {
                    v.push(format!("CreatureDied #{} but creature_died_this_turn is false", object.0));
                }
                let left = after.iter().find(|x| matches!(x, GameEvent::LeftBattlefield { object: o, .. } if o == object));
                match left {
                    Some(GameEvent::LeftBattlefield { to, last_controller, .. }) => {
                        if *to != Zone::Graveyard || last_controller != controller {
                            v.push(format!("CreatureDied #{} (p{}) but it left for {to:?} from p{}", object.0, controller.0, last_controller.0));
                        }
                    }
                    _ => v.push(format!("CreatureDied #{} without leaving the battlefield afterwards (CR 700.4)", object.0)),
                }
                match state.get_object(*object) {
                    None if !is_token => v.push(format!("CreatureDied #{}: a card that ceased to exist", object.0)),
                    Some(o) if o.zone == Zone::Battlefield
                        && !after.iter().any(|x| matches!(x, GameEvent::EnteredBattlefield { object: o2, .. } if o2 == object)) =>
                        v.push(format!("CreatureDied #{} but it is on the battlefield with no re-entry", object.0)),
                    _ => {}
                }
            }
            GameEvent::LeftBattlefield { object, to, .. } => {
                if *to == Zone::Graveyard {
                    if let Some(o) = state.get_object(*object) {
                        if !o.is_token && state.is_creature(*object, registry) {
                            let died_before = events[..i].iter().rev()
                                .take_while(|x| !matches!(x, GameEvent::EnteredBattlefield { object: o2, .. } if o2 == object))
                                .any(|x| matches!(x, GameEvent::CreatureDied { object: o2, .. } if o2 == object));
                            if !died_before {
                                v.push(format!("creature #{} ({}) went to the graveyard without dying (CR 700.4)", object.0, o.name));
                            }
                        }
                    }
                }
                // CR 111.8: a token that left never comes back.
                let is_token = state.get_object(*object).is_none_or(|o| o.is_token);
                if is_token && events[i + 1..].iter().any(|x| matches!(x,
                    GameEvent::EnteredBattlefield { object: o2, .. } | GameEvent::ObjectMoved { object: o2, .. } if o2 == object))
                {
                    v.push(format!("token #{} changed zones again after leaving the battlefield (CR 111.8)", object.0));
                }
            }
            GameEvent::EnteredBattlefield { object, .. } => {
                let Some(o) = state.get_object(*object) else { continue };
                // CR 302.6: a creature that arrived this action is summoning
                // sick unless it was put onto the battlefield attacking, and
                // no untap step has passed since.
                let untapped_since = events[i + 1..].iter().any(|x| matches!(x, GameEvent::StepStarted { step: Step::Untap }));
                let attacking = state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(object));
                // A token put onto the battlefield attacking is exempt, and
                // so is one still being asked whom it attacks.
                let choosing_attack = o.is_token && matches!(&state.awaiting_action,
                    Some(crate::state::AwaitingAction::ResolutionChoice {
                        choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                            effect: crate::state::PendingEffect::TokenAttacks { .. }, .. }, .. }));
                if o.zone == Zone::Battlefield && state.is_creature(*object, registry)
                    && !o.summoning_sick && !attacking && !choosing_attack && !untapped_since
                {
                    v.push(format!("creature #{} ({}) entered this action but is not summoning sick (CR 302.6)", object.0, o.name));
                }
                // CR 306.5b: a planeswalker enters with its printed loyalty.
                let printed = o.copy_grantor.unwrap_or(o.card_id);
                let Some(d) = registry.card_data(printed) else { continue };
                if !d.card_types.contains(&CardType::Planeswalker) {
                    continue;
                }
                let expected = if d.cost.as_ref().is_some_and(|c| c.has_x()) {
                    o.x_value.unwrap_or(0)
                } else {
                    registry.get(printed).and_then(|b| b.starting_loyalty()).unwrap_or(0)
                };
                let loyalty = o.counters.get(&crate::types::CounterType::Loyalty).copied().unwrap_or(0);
                if o.zone != Zone::Battlefield || loyalty != expected {
                    v.push(format!("planeswalker #{} ({}) entered with {loyalty} loyalty in {:?}, expected {expected} (CR 306.5b)",
                        object.0, o.name, o.zone));
                }
            }
            _ => {}
        }
    }

    // The last zone event about an object agrees with where it is now.
    let mut last_zone: std::collections::HashMap<ObjectId, Option<Zone>> = std::collections::HashMap::new();
    for e in events {
        match e {
            GameEvent::EnteredBattlefield { object, .. } => { last_zone.insert(*object, Some(Zone::Battlefield)); }
            GameEvent::LeftBattlefield { object, to, .. } => { last_zone.insert(*object, Some(*to)); }
            GameEvent::ObjectMoved { object, to, .. } => { last_zone.insert(*object, Some(*to)); }
            _ => {}
        }
    }
    for (object, zone) in last_zone {
        if let (Some(o), Some(z)) = (state.get_object(object), zone) {
            if o.zone != z {
                v.push(format!("#{} ({}) last moved to {z:?} but is in {:?}", object.0, o.name, o.zone));
            }
        }
    }

    // CR 701.26: tap and untap are edges, and the last one is the state.
    let mut last: std::collections::HashMap<ObjectId, (bool, usize)> = std::collections::HashMap::new();
    for (i, e) in events.iter().enumerate() {
        match e {
            GameEvent::Tapped { object } | GameEvent::Untapped { object } => {
                let tapped = matches!(e, GameEvent::Tapped { .. });
                if let Some((prev, _)) = last.get(object) {
                    if *prev == tapped {
                        v.push(format!("#{} was {} twice in a row (CR 701.26)", object.0, if tapped { "tapped" } else { "untapped" }));
                    }
                }
                last.insert(*object, (tapped, i));
            }
            GameEvent::LeftBattlefield { object, .. } | GameEvent::EnteredBattlefield { object, .. } => {
                last.remove(object);
            }
            _ => {}
        }
    }
    for (object, (tapped, _)) in last {
        if let Some(o) = state.get_object(object) {
            if o.zone != Zone::Battlefield {
                v.push(format!("#{} was {} but is in {:?}", object.0, if tapped { "tapped" } else { "untapped" }, o.zone));
            } else if o.tapped != tapped {
                v.push(format!("#{} was {} but tapped={} now", object.0, if tapped { "tapped" } else { "untapped" }, o.tapped));
            }
        }
    }
}

fn steps(state: &GameState, registry: &CardRegistry, events: &[GameEvent], v: &mut Violations) {
    // The game's own result events agree with the state.
    let mut ended = 0;
    for e in events {
        match e {
            GameEvent::PlayerLost { player, reason } => {
                if player_ok(state, *player) {
                    let p = state.get_player(*player);
                    if !p.lost || p.loss_reason != Some(reason.clone()) {
                        v.push(format!("PlayerLost p{} ({reason:?}) but lost={} reason={:?}", player.0, p.lost, p.loss_reason));
                    }
                }
            }
            GameEvent::GameEnded { result } => {
                ended += 1;
                if state.result.as_ref() != Some(result) {
                    v.push(format!("GameEnded {result:?} but the result is {:?}", state.result));
                }
            }
            GameEvent::ManaAdded { amount, .. } if *amount == 0 => v.push("ManaAdded of nothing".into()),
            _ => {}
        }
    }
    if ended > 1 {
        v.push(format!("the game ended {ended} times in one action"));
    }
    // CR 500.5/106.4: a step boundary empties the pools, unless mana was
    // added since.
    if let Some(last_step) = events.iter().rposition(|e| matches!(e, GameEvent::StepStarted { .. })) {
        if let GameEvent::StepStarted { step } = &events[last_step] {
            if *step != state.step {
                v.push(format!("the last step to start was {step:?} but the state is in {:?}", state.step));
            }
            if *step == Step::Untap && !events[..last_step].iter().any(|e| matches!(e, GameEvent::TurnStarted { .. })) {
                v.push("an untap step started without a turn starting".into());
            }
        }
        let added_since = events[last_step..].iter().any(|e| matches!(e, GameEvent::ManaAdded { .. }));
        if !added_since {
            for p in &state.players {
                if !p.mana_pool.is_empty() {
                    v.push(format!("p{} has mana floating across a step boundary (CR 500.5)", p.id.0));
                }
            }
        }
        // CR 500.2/405.6e: a step begins on an empty stack; only triggers
        // can have joined since.
        if state.stack.iter().any(|s| !matches!(s, StackEntry::Trigger(_))) {
            v.push("a spell or ability survived a step boundary (CR 500.2)".into());
        }
        if state.resolving_spell.is_some() || state.pending_spell_cast.is_some() || state.pending_ability_effect.is_some() {
            v.push("a cast or resolution straddles a step boundary".into());
        }
        if state.objects_in_id_order().iter().any(|o| o.zone == Zone::Stack) {
            v.push("an object is in the stack zone right after a step change".into());
        }
        if state.consecutive_passes != 0 {
            v.push(format!("{} passes carried across a step boundary", state.consecutive_passes));
        }
        // CR 504.1: the draw step draws exactly one card for the active player.
        let draw_started = matches!(events[last_step], GameEvent::StepStarted { step: Step::Draw });
        if draw_started && state.result.is_none()
            && player_ok(state, state.active_player) && !state.get_player(state.active_player).lost
        {
            let draws: Vec<PlayerId> = events[last_step..].iter().filter_map(|e| match e {
                GameEvent::CardDrawn { player, .. } => Some(*player), _ => None }).collect();
            if draws != vec![state.active_player] {
                v.push(format!("the draw step drew {draws:?} for p{} (CR 504.1)", state.active_player.0));
            }
        }
    }

    // CR 502.3/514.2/505.6b: a turn begins clean.
    let Some((player, turn)) = events.iter().find_map(|e| match e {
        GameEvent::TurnStarted { player, turn } => Some((*player, *turn)), _ => None }) else { return };
    let w = format!("turn {turn} start");
    if player != state.active_player || turn != state.turn_number
        || !matches!(state.step, Step::Untap | Step::Upkeep | Step::PrecombatMain)
    {
        v.push(format!("{w}: p{} is active on turn {} in {:?}", state.active_player.0, state.turn_number, state.step));
    }
    for obj in state.objects_in_id_order() {
        if !obj.abilities_activated_this_turn.is_empty() {
            v.push(format!("{w}: {} (#{}) remembers activations from last turn", obj.name, obj.id.0));
        }
        if obj.zone != Zone::Battlefield {
            continue;
        }
        if obj.damage_marked != 0 || !obj.damaged_by.is_empty() || obj.dealt_deathtouch_damage {
            v.push(format!("{w}: {} (#{}) carries damage from last turn (CR 514.2)", obj.name, obj.id.0));
        }
        if obj.regeneration_shields != 0 {
            v.push(format!("{w}: {} (#{}) keeps a regeneration shield (CR 514.2)", obj.name, obj.id.0));
        }
        if obj.controller == state.active_player {
            if obj.tapped && state.untaps_normally(obj.id, registry) {
                v.push(format!("{w}: {} (#{}) did not untap (CR 502.3)", obj.name, obj.id.0));
            }
            if obj.summoning_sick {
                v.push(format!("{w}: {} (#{}) is summoning sick at the start of its controller's turn", obj.name, obj.id.0));
            }
        }
    }
    if !state.until_end_of_turn.is_empty() {
        v.push(format!("{w}: {} until-end-of-turn effects survive (CR 514.2)", state.until_end_of_turn.len()));
    }
    if state.creature_died_this_turn || state.num_spells_cast_this_turn.values().any(|n| *n != 0) {
        v.push(format!("{w}: per-turn counters not reset"));
    }
    if state.combat.is_some() || state.combat_damage_step_pending || !state.end_of_combat_exiles.is_empty() {
        v.push(format!("{w}: combat state survives"));
    }
    if player_ok(state, state.active_player) && state.get_player(state.active_player).land_plays_remaining != 1 {
        v.push(format!("{w}: the land drop was not reset (CR 305.2)"));
    }
    for p in &state.players {
        if !p.mana_pool.is_empty() {
            v.push(format!("{w}: p{} has mana floating", p.id.0));
        }
    }
    let prev = state.opponent(state.active_player);
    let hand = state.objects_in_zone(Zone::Hand, prev).len();
    if hand > 7 {
        v.push(format!("{w}: p{} holds {hand} cards after their cleanup (CR 514.1)", prev.0));
    }
}

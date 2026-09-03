//! Turn structure, the game result, and combat bookkeeping (CR 104, 500–511).

use super::{player_ok, Violations};
use crate::cards::CardRegistry;
use crate::events::LossReason;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameResult, GameState, TemporaryEffect};
use crate::types::{CardType, Step, Zone};

pub(super) fn check_core(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    // CR 103.7a/500: the turn counter and its first-turn flag agree.
    if state.turn_number < 1 {
        v.push("turn_number is 0".into());
    }
    if state.is_first_turn != (state.turn_number == 1) {
        v.push(format!("is_first_turn={} on turn {}", state.is_first_turn, state.turn_number));
    }
    if state.turn_number == 1 && state.step == Step::Draw && state.players.len() == 2 {
        v.push("a draw step on the first turn (CR 103.7a)".into());
    }
    // CR 502.4: no priority in the untap step.
    if state.step == Step::Untap && state.priority_player.is_some() {
        v.push("a player holds priority in the untap step (CR 502.4)".into());
    }

    // Two-player engine: exactly two players.
    if state.players.len() != 2 {
        v.push(format!("{} players in a two-player engine", state.players.len()));
    }
    // CR 104: losing is recorded with its reason.
    for p in &state.players {
        if p.lost != p.loss_reason.is_some() {
            v.push(format!("p{}: lost={} but loss_reason={:?}", p.id.0, p.lost, p.loss_reason));
        }
        // CR 305.2: one land per turn in this pool.
        if p.land_plays_remaining > 1 {
            v.push(format!("p{} has {} land plays remaining", p.id.0, p.land_plays_remaining));
        }
    }

    // Every player id stored in game-level bookkeeping names a player.
    if let Some(c) = &state.combat {
        for (a, d) in &c.attackers {
            if !player_ok(state, *d) {
                v.push(format!("attacker #{} attacks p{} who is not a player", a.0, d.0));
            }
        }
    }
    for e in &state.control_effects {
        for p in [e.controller, e.original_controller, e.source_controller] {
            if !player_ok(state, p) {
                v.push(format!("control effect over #{} names p{} who is not a player", e.object.0, p.0));
            }
        }
    }
    for (p, _) in &state.pending_mulligan_bottoms {
        if !player_ok(state, *p) {
            v.push(format!("queued bottoming for p{} who is not a player", p.0));
        }
    }
    for p in state.num_spells_cast_this_turn.keys().chain(state.num_spells_cast_last_turn.keys()) {
        if !player_ok(state, *p) {
            v.push(format!("spell count for p{} who is not a player", p.0));
        }
    }
    for e in &state.until_end_of_turn {
        if let TemporaryEffect::ChangeControl { target, original_controller } = e {
            if !player_ok(state, *original_controller) {
                v.push(format!("control change of #{} from p{} who is not a player", target.0, original_controller.0));
            }
        }
    }
    if let Some(AwaitingAction::DeclareBlockers { defending_player }) = &state.awaiting_action {
        if !player_ok(state, *defending_player) {
            v.push(format!("blockers prompt for p{} who is not a player", defending_player.0));
        }
    }

    // CR 603.7: the delayed end-of-combat exiles are well-formed, and (every
    // creator in this pool is an attack trigger) exist only inside combat.
    for e in &state.end_of_combat_exiles {
        if !player_ok(state, e.controller) {
            v.push(format!("end-of-combat exile of #{} for p{} who is not a player", e.target_id.0, e.controller.0));
        }
        if registry.get(e.source_card_id).is_none() {
            v.push(format!("end-of-combat exile of #{} from unregistered card {}", e.target_id.0, e.source_card_id.0));
        }
        if e.target_id == e.source_id {
            v.push(format!("#{} schedules its own end-of-combat exile", e.source_id.0));
        }
    }
    if !state.end_of_combat_exiles.is_empty()
        && (state.combat.is_none()
            || !matches!(state.step, Step::DeclareAttackers | Step::DeclareBlockers | Step::CombatDamage))
    {
        v.push(format!("end-of-combat exiles scheduled outside combat ({:?})", state.step));
    }
}

pub(super) fn check_settled(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    let n = state.players.len();
    // CR 117.4: a full round of passes has been acted on before anyone is asked again.
    if state.consecutive_passes as usize >= n {
        v.push(format!("{} consecutive passes with {n} players and nothing resolved or advanced (CR 117.4)",
            state.consecutive_passes));
    }

    // CR 104.2a/104.4a: a loss ends the game; a result matches the losses.
    let any_lost = state.players.iter().any(|p| p.lost);
    if any_lost && state.result.is_none() {
        v.push("a player has lost but the game has no result (CR 104.2a)".into());
    }
    match state.result {
        Some(GameResult::Winner(w)) => {
            if !player_ok(state, w) {
                v.push(format!("winner p{} is not a player", w.0));
            } else if state.get_player(w).lost || state.players.iter().any(|p| p.id != w && !p.lost) {
                v.push(format!("p{} is the winner but the loss flags say otherwise", w.0));
            }
        }
        Some(GameResult::Draw) => {
            if state.players.iter().any(|p| !p.lost) {
                v.push("a draw with a player who has not lost (CR 104.4a)".into());
            }
        }
        None => {}
    }
    for p in &state.players {
        if p.loss_reason == Some(LossReason::OpponentWon)
            && state.result != Some(GameResult::Winner(state.opponent(p.id)))
        {
            v.push(format!("p{} lost because the opponent won, but the result is {:?}", p.id.0, state.result));
        }
    }
    // A player who has left the game is never asked anything (CR 104.3a).
    if let Some(p) = state.priority_player {
        if player_ok(state, p) && state.get_player(p).lost {
            v.push(format!("p{} holds priority after losing", p.0));
        }
    }
    let prompted = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, .. })
        | Some(AwaitingAction::DiscardToHandSize { player, .. })
        | Some(AwaitingAction::MulliganDecision { player })
        | Some(AwaitingAction::BottomAfterMulligan { player, .. }) => Some(*player),
        Some(AwaitingAction::DeclareBlockers { defending_player }) => Some(*defending_player),
        Some(AwaitingAction::DeclareAttackers) => Some(state.active_player),
        None => None,
    };
    if let Some(p) = prompted {
        if player_ok(state, p) && state.get_player(p).lost {
            v.push(format!("p{} is prompted after losing", p.0));
        }
    }

    // ── Combat is step-gated (CR 506.1, 508.8, 510.4, 511) ─────────────
    if state.combat_damage_step_pending && (state.step != Step::CombatDamage || state.combat.is_none()) {
        v.push(format!("second combat damage step pending in {:?} (CR 510.4)", state.step));
    }
    if matches!(state.step, Step::DeclareBlockers | Step::CombatDamage)
        && !state.combat.as_ref().is_some_and(|c| c.any_attackers_declared)
    {
        v.push(format!("{:?} reached without attackers declared (CR 508.8)", state.step));
    }
    if state.step == Step::DeclareAttackers && state.combat.is_none()
        && !matches!(state.awaiting_action, Some(AwaitingAction::DeclareAttackers))
    {
        v.push("declare attackers step past its declaration with no combat state".into());
    }
    let Some(c) = &state.combat else { return };
    if !c.dealt_first_strike.is_empty() && state.step != Step::CombatDamage {
        v.push(format!("first-strike damage recorded in {:?}", state.step));
    }
    if !c.attackers.is_empty() && !c.any_attackers_declared {
        v.push("attackers in combat but none declared".into());
    }
    // CR 506.3/506.4: every combatant is a creature on the battlefield.
    let participants: Vec<(&str, ObjectId)> = c.attackers.keys().map(|id| ("attacker", *id))
        .chain(c.blocker_assignments.keys().map(|id| ("attacked", *id)))
        .chain(c.blocker_assignments.values().flatten().map(|id| ("blocker", *id)))
        .chain(c.blocked_attackers.iter().map(|id| ("blocked attacker", *id)))
        .chain(c.planeswalker_defenders.keys().map(|id| ("planeswalker attacker", *id)))
        .collect();
    for (role, id) in participants {
        match state.get_object(id) {
            Some(o) if o.zone == Zone::Battlefield && state.is_creature(id, registry) => {}
            Some(o) => v.push(format!("{role} #{} is in {:?} / not a creature but still in combat (CR 506.4)", id.0, o.zone)),
            None => v.push(format!("{role} #{} does not exist but is still in combat", id.0)),
        }
    }
    // CR 509.1h: an attacker with blockers is blocked.
    for (a, blockers) in &c.blocker_assignments {
        if !blockers.is_empty() && !c.blocked_attackers.contains(a) {
            v.push(format!("attacker #{} has blockers but is not marked blocked (CR 509.1h)", a.0));
        }
    }
    // CR 506.2: the non-active player is the only defending player.
    let def = state.opponent(state.active_player);
    for (a, d) in &c.attackers {
        if *d != def {
            v.push(format!("attacker #{} attacks p{}, not the defending player p{} (CR 506.2)", a.0, d.0, def.0));
        }
    }
    for (a, w) in &c.planeswalker_defenders {
        if a == w {
            v.push(format!("#{} attacks itself", a.0));
        }
        if let Some(o) = state.get_object(*w) {
            if o.zone == Zone::Battlefield
                && (!state.has_card_type(*w, CardType::Planeswalker, registry) || o.controller != def)
            {
                v.push(format!("#{} attacks #{} which is not a planeswalker of the defending player (CR 506.2)", a.0, w.0));
            }
        }
    }
}

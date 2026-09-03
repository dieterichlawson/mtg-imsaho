//! The shape of every prompt the engine can park in `awaiting_action`: a
//! choice offers real things (CR 608.2d, 101.4), and a turn-based-action
//! prompt sits in the step that raises it with the state that step
//! guarantees (CR 703, 508.1, 509.1, 514.1, 103.4). Core tier: a prompt is
//! its own decision point and nothing moves while it is up.

use super::{player_ok, Violations};
use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind, LONDON_MULLIGAN_CAP};
use crate::types::{ManaSymbol, Step, Zone};

fn distinct(ids: &[ObjectId], what: &str, v: &mut Violations) {
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if !seen.insert(*id) {
            v.push(format!("{what} lists #{} twice", id.0));
        }
    }
}

fn on_battlefield(state: &GameState, id: ObjectId) -> bool {
    state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield)
}

fn pools_empty(state: &GameState, what: &str, v: &mut Violations) {
    for p in &state.players {
        if !p.mana_pool.is_empty() {
            v.push(format!("{what}: p{} has mana floating (CR 500.5)", p.id.0));
        }
    }
}

pub(super) fn check_core(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    let Some(awaiting) = &state.awaiting_action else { return };
    let active = state.active_player;
    match awaiting {
        AwaitingAction::ResolutionChoice { player, source, choice } => {
            check_choice(state, registry, *player, *source, choice, v);
        }
        AwaitingAction::DeclareAttackers => {
            let w = "attackers prompt";
            if state.step != Step::DeclareAttackers {
                v.push(format!("{w} in {:?} (CR 508.1)", state.step));
            }
            if state.combat.is_some() {
                v.push(format!("{w} with combat state already present"));
            }
            if state.priority_player != Some(active) {
                v.push(format!("{w} but priority is {:?}, not the active player's", state.priority_player));
            }
            tba_common(state, w, v);
            if state.combat_damage_step_pending || !state.end_of_combat_exiles.is_empty() {
                v.push(format!("{w} with leftovers from a previous combat"));
            }
        }
        AwaitingAction::DeclareBlockers { defending_player } => {
            let w = "blockers prompt";
            let d = *defending_player;
            if state.step != Step::DeclareBlockers {
                v.push(format!("{w} in {:?} (CR 509.1)", state.step));
            }
            if !player_ok(state, d) || d != state.opponent(active) {
                v.push(format!("{w} for p{} who is not the defending player (CR 506.2)", d.0));
            }
            if state.priority_player != Some(d) {
                v.push(format!("{w} but priority is {:?}, not the defender's", state.priority_player));
            }
            tba_common(state, w, v);
            match &state.combat {
                None => v.push(format!("{w} with no combat")),
                Some(c) => {
                    if c.attackers.is_empty() || !c.any_attackers_declared {
                        v.push(format!("{w} with no attackers declared (CR 508.8)"));
                    }
                    if c.attackers.values().any(|p| *p != d) {
                        v.push(format!("{w} but an attacker attacks someone other than p{}", d.0));
                    }
                    if c.blocker_assignments.values().any(|b| !b.is_empty())
                        || !c.blocked_attackers.is_empty() || !c.dealt_first_strike.is_empty()
                    {
                        v.push(format!("{w} but blocks or damage are already recorded"));
                    }
                }
            }
            if state.combat_damage_step_pending {
                v.push(format!("{w} with a second damage step pending"));
            }
        }
        AwaitingAction::DiscardToHandSize { player, discard_count } => {
            let w = "discard-to-hand-size prompt";
            if state.step != Step::Cleanup {
                v.push(format!("{w} in {:?} (CR 514.1)", state.step));
            }
            if *player != active || state.priority_player != Some(active) {
                v.push(format!("{w} for p{} with priority {:?}; it is p{}'s cleanup", player.0, state.priority_player, active.0));
            }
            tba_common(state, w, v);
            if state.combat.is_some() {
                v.push(format!("{w} with combat state present"));
            }
            if player_ok(state, *player) {
                let hand = state.objects_in_zone(Zone::Hand, *player).len();
                if *discard_count == 0 || hand != discard_count + 7 {
                    v.push(format!("{w} asks for {discard_count} discards from a hand of {hand} (CR 514.1)"));
                }
            }
        }
        AwaitingAction::MulliganDecision { player } | AwaitingAction::BottomAfterMulligan { player, .. } => {
            mulligan_shape(state, v);
            if player_ok(state, *player) {
                let ps = state.get_player(*player);
                let hand = state.objects_in_zone(Zone::Hand, *player).len();
                if hand != 7 {
                    v.push(format!("mulligan prompt for p{} holding {hand} cards (CR 103.5)", player.0));
                }
                match awaiting {
                    AwaitingAction::MulliganDecision { .. } => {
                        if ps.mulligan_kept {
                            v.push(format!("keep/mulligan prompt for p{} who already kept", player.0));
                        }
                    }
                    AwaitingAction::BottomAfterMulligan { count, .. } => {
                        if !ps.mulligan_kept || *count != ps.mulligan_count as usize
                            || *count == 0 || *count > LONDON_MULLIGAN_CAP as usize
                        {
                            v.push(format!("bottoming prompt for p{}: bottom {count} after {} mulligans, kept={}",
                                player.0, ps.mulligan_count, ps.mulligan_kept));
                        }
                        if state.players.iter().any(|p| !p.mulligan_kept) {
                            v.push("bottoming started before every player kept (CR 103.5)".into());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// What every in-turn turn-based-action prompt guarantees (CR 703.3, 500.2).
fn tba_common(state: &GameState, w: &str, v: &mut Violations) {
    if !state.stack.is_empty() {
        v.push(format!("{w} with {} entries on the stack (CR 500.2)", state.stack.len()));
    }
    if !state.pending_triggers.is_empty() || !state.pending_trigger_pushes_ap.is_empty()
        || !state.pending_trigger_pushes_nap.is_empty()
    {
        v.push(format!("{w} with triggers still queued"));
    }
    if state.pending_spell_cast.is_some() || state.pending_ability_effect.is_some() || state.resolving_spell.is_some() {
        v.push(format!("{w} with a cast or resolution in progress"));
    }
    pools_empty(state, w, v);
}

/// CR 103: the opening-hand phase is turn 1 before anything has happened.
fn mulligan_shape(state: &GameState, v: &mut Violations) {
    let w = "mulligan phase";
    if state.turn_number != 1 || !state.is_first_turn || state.step != Step::Untap {
        v.push(format!("{w} on turn {} in {:?}", state.turn_number, state.step));
    }
    if state.priority_player.is_some() {
        v.push(format!("{w} with a priority holder"));
    }
    if !state.stack.is_empty() || state.combat.is_some() || state.result.is_some() {
        v.push(format!("{w} with a stack, combat, or a result"));
    }
    if !state.until_end_of_turn.is_empty() || !state.control_effects.is_empty() {
        v.push(format!("{w} with effects in force"));
    }
    tba_common(state, w, v);
    for obj in state.objects_in_id_order() {
        if obj.is_token || !matches!(obj.zone, Zone::Library | Zone::Hand) {
            v.push(format!("{w}: {} (#{}) is a {} in {:?}", obj.name, obj.id.0,
                if obj.is_token { "token" } else { "card" }, obj.zone));
        }
    }
    for p in &state.players {
        if p.lost || p.has_drawn_from_empty || p.land_plays_remaining != 1 || p.mulligan_count > LONDON_MULLIGAN_CAP {
            v.push(format!("{w}: p{} already has turn state (lost={}, drew from empty={}, land plays={}, mulligans={})",
                p.id.0, p.lost, p.has_drawn_from_empty, p.land_plays_remaining, p.mulligan_count));
        }
    }
    for (p, c) in &state.pending_mulligan_bottoms {
        if !player_ok(state, *p) || *c > LONDON_MULLIGAN_CAP as usize {
            v.push(format!("{w}: queued bottoming of {c} for p{}", p.0));
        }
    }
}

fn check_choice(state: &GameState, registry: &CardRegistry, player: crate::ids::PlayerId, source: ObjectId,
                choice: &ResolutionChoiceKind, v: &mut Violations) {
    use ResolutionChoiceKind as K;
    match choice {
        K::ChooseTarget { options, optional, effect, .. } => {
            let w = "target prompt";
            for (i, t) in options.iter().enumerate() {
                match t {
                    Target::Object(id) if state.get_object(*id).is_none() => v.push(format!("{w} offers missing #{}", id.0)),
                    Target::Player(p) if !player_ok(state, *p) => v.push(format!("{w} offers p{} who is not a player", p.0)),
                    Target::Illegal => v.push(format!("{w} offers an Illegal target")),
                    _ => {}
                }
                if options[..i].contains(t) {
                    v.push(format!("{w} offers {t:?} twice"));
                }
            }
            match effect {
                PendingEffect::LegendRuleKeep { player: p2, legend_name } => {
                    // CR 704.5j: the prompt is exactly the duplicate group.
                    if *p2 != player || *optional {
                        v.push(format!("legend-rule prompt for p{} answered by p{}, optional={optional}", p2.0, player.0));
                    }
                    let group: std::collections::BTreeSet<ObjectId> = state.objects_in_id_order().into_iter()
                        .filter(|o| o.zone == Zone::Battlefield && o.controller == player
                            && state.name_of(o.id, registry) == *legend_name && state.is_legendary(o.id, registry))
                        .map(|o| o.id)
                        .collect();
                    let offered: std::collections::BTreeSet<ObjectId> = options.iter()
                        .filter_map(|t| match t { Target::Object(id) => Some(*id), _ => None })
                        .collect();
                    if offered.len() != options.len() || offered != group || group.len() < 2 {
                        v.push(format!("legend-rule prompt for {legend_name:?} offers {offered:?} but the duplicate group is {group:?} (CR 704.5j)"));
                    }
                }
                PendingEffect::AttachTargetToPendingTrigger => {
                    // CR 603.3d: the prompt is for the trigger the answer will pop.
                    let q = if state.pending_trigger_pushes_ap.is_empty() {
                        &state.pending_trigger_pushes_nap
                    } else {
                        &state.pending_trigger_pushes_ap
                    };
                    match q.first() {
                        None => v.push("trigger-target prompt with no queued trigger".into()),
                        Some(t) => {
                            if t.source.id != source || t.source.controller != player || !t.source.chosen_targets.is_empty() {
                                v.push(format!("trigger-target prompt for #{} (p{}) but the queue's front is #{} (p{}, {} targets)",
                                    source.0, player.0, t.source.id.0, t.source.controller.0, t.source.chosen_targets.len()));
                            }
                        }
                    }
                    if *optional || options.len() < 2 {
                        v.push(format!("trigger-target prompt with {} options, optional={optional}", options.len()));
                    }
                    if player != state.active_player && !state.pending_trigger_pushes_ap.is_empty() {
                        v.push("the non-active player is choosing a trigger target while the active player's triggers wait (CR 603.3b)".into());
                    }
                }
                PendingEffect::TokenAttacks { token_id, remaining, .. } => {
                    if !state.get_object(*token_id).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player) {
                        v.push(format!("token-attacks prompt for #{} which p{} does not control on the battlefield", token_id.0, player.0));
                    }
                    if state.combat.is_none() {
                        v.push("token-attacks prompt outside combat".into());
                    }
                    distinct(remaining, "token-attacks prompt", v);
                    if remaining.contains(token_id) {
                        v.push("token-attacks prompt lists the token among the remaining ones".into());
                    }
                }
                PendingEffect::FinishLibrarySearch { searcher, .. } => {
                    for t in options {
                        match t {
                            Target::Object(id) => library_option(state, *searcher, *id, "library search", v),
                            other => v.push(format!("library search offers {other:?}")),
                        }
                    }
                }
                _ => {}
            }
        }
        K::ChooseCardFromHand { player: p, cards, remaining, .. } => {
            let w = "hand prompt";
            if *p != player {
                v.push(format!("{w} for p{} answered by p{}", p.0, player.0));
            }
            if *remaining == 0 {
                v.push(format!("{w} with nothing left to choose"));
            }
            distinct(cards, w, v);
            for c in cards {
                if !state.get_object(*c).is_some_and(|o| o.zone == Zone::Hand && o.owner == *p) {
                    v.push(format!("{w} offers #{} which is not in p{}'s hand", c.0, p.0));
                }
            }
        }
        K::ChooseFromLibrary { options, searcher, .. } => {
            distinct(options, "library prompt", v);
            for o in options {
                library_option(state, *searcher, *o, "library prompt", v);
            }
        }
        K::ChooseFromRevealed { revealed, .. } => {
            distinct(revealed, "revealed prompt", v);
            for r in revealed {
                match state.get_object(*r) {
                    None => v.push(format!("revealed prompt offers missing #{}", r.0)),
                    Some(o) if o.is_token => v.push(format!("revealed prompt offers token #{}", r.0)),
                    _ => {}
                }
            }
        }
        K::DividePermanentsIntoPiles { permanents, target_player, .. } => {
            let w = "pile prompt";
            distinct(permanents, w, v);
            if !player_ok(state, *target_player) {
                v.push(format!("{w} for p{} who is not a player", target_player.0));
            }
            for id in permanents {
                if !state.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == *target_player) {
                    v.push(format!("{w} lists #{} which p{} does not control on the battlefield (CR 700.3c)", id.0, target_player.0));
                }
            }
        }
        K::ChoosePile { pile_1, pile_2, .. } => {
            let w = "pile choice";
            distinct(pile_1, w, v);
            distinct(pile_2, w, v);
            for id in pile_1 {
                if pile_2.contains(id) {
                    v.push(format!("{w}: #{} is in both piles (CR 700.3a)", id.0));
                }
            }
            for id in pile_1.iter().chain(pile_2) {
                if !on_battlefield(state, *id) {
                    v.push(format!("{w}: #{} is not on the battlefield (CR 700.3c)", id.0));
                }
            }
            if pile_1.is_empty() && pile_2.is_empty() {
                v.push(format!("{w} between two empty piles"));
            }
        }
        K::PayOrNot { spell_id, source_spell_id, cost, .. } => {
            // The spell asked about is on the stack; the one asking is resolving.
            if !state.stack.iter().any(|e| e.as_spell() == Some(*spell_id)) {
                v.push(format!("pay-or-not prompt for #{} which is not on the stack", spell_id.0));
            }
            if state.resolving_spell != Some(*source_spell_id) {
                v.push(format!("pay-or-not prompt from #{} which is not the resolving spell", source_spell_id.0));
            }
            if cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X)) {
                v.push("pay-or-not prompt with an unannounced X".into());
            }
        }
        K::ChooseTriggerOrder { options, ap_queue, indices, .. } => {
            let w = "trigger-order prompt";
            let q = if *ap_queue { &state.pending_trigger_pushes_ap } else { &state.pending_trigger_pushes_nap };
            if indices.len() != options.len() || indices.len() < 2 {
                v.push(format!("{w} with {} options for {} indices", options.len(), indices.len()));
            }
            if indices.windows(2).any(|w2| w2[0] >= w2[1]) {
                v.push(format!("{w} indices {indices:?} are not increasing"));
            }
            for &i in indices {
                match q.get(i) {
                    None => v.push(format!("{w} index {i} is past the queue of {}", q.len())),
                    Some(t) if t.source.controller != player =>
                        v.push(format!("{w} for p{} orders p{}'s trigger", player.0, t.source.controller.0)),
                    _ => {}
                }
            }
            if let Some(t) = indices.first().and_then(|&i| q.get(i)) {
                if t.source.id != source {
                    v.push(format!("{w} names source #{} but its first trigger is from #{}", source.0, t.source.id.0));
                }
            }
            if *ap_queue != (player == state.active_player) {
                v.push(format!("{w}: ap_queue={ap_queue} for p{} while p{} is active (CR 603.3b)", player.0, state.active_player.0));
            }
            if !*ap_queue && !state.pending_trigger_pushes_ap.is_empty() {
                v.push(format!("{w} for the non-active player while active-player triggers wait (CR 603.3b)"));
            }
        }
        K::ChooseCardType { options, .. } | K::ChooseCardName { options, .. } => {
            if options.is_empty() {
                v.push("a name/type prompt offers nothing".into());
            }
        }
        // Stash links and exile options live with the stack checks.
        K::ChooseXFunding { .. } | K::ChooseExileFromGraveyard { .. } | K::YesNo { .. } => {}
    }
}

fn library_option(state: &GameState, searcher: crate::ids::PlayerId, id: ObjectId, w: &str, v: &mut Violations) {
    if !player_ok(state, searcher) {
        v.push(format!("{w} for p{} who is not a player", searcher.0));
        return;
    }
    let listed = state.get_player(searcher).library_order.contains(&id);
    let ok = state.get_object(id).is_some_and(|o| o.zone == Zone::Library && o.owner == searcher) && listed;
    if !ok {
        v.push(format!("{w} offers #{} which is not in p{}'s library (CR 701.23a)", id.0, searcher.0));
    }
}

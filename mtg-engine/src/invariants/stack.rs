//! The stack, the trigger queues, and the two "cast in progress" stashes:
//! CR 601–603 and 608 as bookkeeping. Everything here is atomic with
//! respect to decision points, so it holds at the core tier.

use super::{player_ok, Violations};
use crate::actions::Target;
use crate::cards::{CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind, StackEntry};
use crate::triggers::{PendingTrigger, TriggerEvent};
use crate::types::{CardType, Keyword, Zone};

/// CR 115.3 and 608.2b: a stored target list names real players, never the
/// same target twice, and never carries the resolution-time `Illegal`
/// marker (that is substituted into local copies only).
fn check_targets(state: &GameState, what: &str, targets: &[Target], v: &mut Violations) {
    for (i, t) in targets.iter().enumerate() {
        match t {
            Target::Illegal => v.push(format!("{what} stores an Illegal target (CR 608.2b is decided at resolution)")),
            Target::Player(p) if !player_ok(state, *p) => v.push(format!("{what} targets p{} who is not a player", p.0)),
            _ => {}
        }
        if !matches!(t, Target::Illegal) && targets[..i].contains(t) {
            v.push(format!("{what} targets {t:?} twice (CR 115.3)"));
        }
    }
}

/// Whether `n` chosen targets is a count the requirement allows (CR 601.2c).
pub(super) fn arity_ok(req: &TargetRequirement, n: usize) -> bool {
    match req {
        TargetRequirement::None => n == 0,
        TargetRequirement::UpToTargets(k, _) => n <= *k,
        TargetRequirement::TwoTargets(a, b) => (0..=n).any(|x| arity_ok(a, x) && arity_ok(b, n - x)),
        TargetRequirement::ModalChoice(modes) => modes.iter().any(|m| arity_ok(m, n)),
        _ => n == 1,
    }
}

fn check_trigger(state: &GameState, registry: &CardRegistry, where_: &str, t: &PendingTrigger, v: &mut Violations) {
    let src = &t.source;
    let what = format!("{where_} trigger of {} (#{})", src.description, src.id.0);
    if !player_ok(state, src.controller) {
        v.push(format!("{what} is controlled by p{} who is not a player", src.controller.0));
    }
    check_targets(state, &what, &src.chosen_targets, v);
    // A trigger is given at most one target, and only when the ability that
    // triggered declares a requirement (CR 603.3d).
    if src.chosen_targets.len() > 1 {
        v.push(format!("{what} has {} targets", src.chosen_targets.len()));
    }
    let Some(behavior) = registry.get(src.card_id) else {
        // The engine's own delayed exile trigger needs no behavior; nothing
        // else can resolve without one (CR 608.2c).
        if !matches!(t.event, TriggerEvent::DelayedTokenExile { .. }) {
            v.push(format!("{what} has no behavior in the registry (card {})", src.card_id.0));
        }
        return;
    };
    if !src.chosen_targets.is_empty() {
        let defs = if src.from_back_face {
            behavior.back_face_data().map(|d| d.triggered_abilities).unwrap_or_default()
        } else {
            behavior.card_data().triggered_abilities
        };
        let kind = t.event.kind();
        let targeted = kind.is_some() && defs.iter().any(|d|
            kind.as_ref() == Some(&d.kind) && d.target_requirement.is_some());
        if !targeted {
            v.push(format!("{what} carries a target but the ability does not target"));
        }
    }
}

pub(super) fn check_core(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    // ── Stack entries ──────────────────────────────────────────────────
    let mut spell_entries = std::collections::HashSet::new();
    for (i, entry) in state.stack.iter().enumerate() {
        if let Some(id) = entry.as_spell() {
            if !spell_entries.insert(id) {
                v.push(format!("#{} is on two stack entries", id.0));
            }
        }
        match entry {
            StackEntry::Spell(id) => {
                let Some(obj) = state.get_object(*id) else { continue }; // reported by the base check
                let what = format!("spell {} (#{})", obj.name, id.0);
                check_targets(state, &what, &obj.targets, v);
                let Some(behavior) = registry.get(obj.card_id) else { continue };
                let req = behavior.target_requirement();
                if !arity_ok(&req, obj.targets.len()) {
                    v.push(format!("{what} has {} targets for requirement {req:?} (CR 601.2c)", obj.targets.len()));
                }
                // CR 303.4a/115.1b: an Aura spell has exactly one target, of
                // the kind its enchant ability names.
                if state.has_subtype(*id, "Aura", registry) {
                    match (&req, obj.targets.first()) {
                        (_, None) => v.push(format!("{what} is an Aura spell with no target (CR 303.4a)")),
                        (TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly, Some(Target::Object(_))) =>
                            v.push(format!("{what} enchants players but targets an object")),
                        (TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_)
                            | TargetRequirement::PermanentWithFilter(_), Some(Target::Player(_))) =>
                            v.push(format!("{what} enchants permanents but targets a player")),
                        _ => {}
                    }
                    if obj.targets.len() != 1 {
                        v.push(format!("{what} is an Aura spell with {} targets (CR 303.4a)", obj.targets.len()));
                    }
                }
                // CR 601.2b/107.3a: an X spell announced X — for the cost it
                // was cast with, the flashback cost included (Devil's Play).
                let paid = if obj.cast_with_flashback {
                    registry.card_data(obj.card_id).and_then(|d| d.flashback_cost).or_else(|| state.until_end_of_turn.iter()
                        .find_map(|e| match e {
                            crate::state::TemporaryEffect::GrantFlashback { target, cost } if *target == obj.id => Some(cost.clone()),
                            _ => None,
                        }))
                } else {
                    registry.card_data(obj.card_id).and_then(|d| d.cost)
                };
                if paid.is_some_and(|c| c.has_x()) && obj.x_value.is_none() {
                    v.push(format!("{what} has an X cost but no X announced (CR 601.2b)"));
                }
                // CR 302.1/303.1/307.1 etc.: a sorcery-speed spell was cast
                // onto an empty stack in its controller's main phase, and
                // stays at the bottom until it resolves.
                if let Some(d) = registry.card_data(obj.card_id) {
                    if !d.card_types.contains(&CardType::Instant) && !d.keywords.contains(&Keyword::Flash) {
                        if i != 0 {
                            v.push(format!("{what} is sorcery-speed but sits above {i} stack entries"));
                        }
                        if !state.step.is_main_phase() {
                            v.push(format!("{what} is sorcery-speed on the stack in {:?}", state.step));
                        }
                        if obj.controller != state.active_player {
                            v.push(format!("{what} is sorcery-speed on the stack on p{}'s turn", state.active_player.0));
                        }
                    }
                }
            }
            StackEntry::Trigger(t) => check_trigger(state, registry, "stack", t, v),
            StackEntry::Ability { source_id, activator, targets, target_requirement, behavior_card_id, sacrificed, sacrificed_toughness, .. } => {
                let what = format!("ability of #{} on the stack", source_id.0);
                if sacrificed.is_none() && sacrificed_toughness.is_some() {
                    v.push(format!("{what} remembers a sacrificed creature's toughness but no sacrifice"));
                }
                if !player_ok(state, *activator) {
                    v.push(format!("{what} was activated by p{} who is not a player", activator.0));
                }
                check_targets(state, &what, targets, v);
                match target_requirement {
                    None if !targets.is_empty() => v.push(format!("{what} has targets but no requirement")),
                    Some(req) if !arity_ok(req, targets.len()) =>
                        v.push(format!("{what} has {} targets for requirement {req:?} (CR 602.2b)", targets.len())),
                    _ => {}
                }
                if registry.get(*behavior_card_id).is_none() {
                    v.push(format!("{what} has no behavior in the registry (card {})", behavior_card_id.0));
                }
            }
        }
    }

    // ── Trigger queues (CR 603.3b, 603.8) ───────────────────────────────
    for t in &state.pending_trigger_pushes_ap {
        check_trigger(state, registry, "AP-queued", t, v);
        if t.source.controller != state.active_player {
            v.push(format!("AP push queue holds p{}'s trigger {} (CR 603.3b)", t.source.controller.0, t.source.description));
        }
    }
    for t in &state.pending_trigger_pushes_nap {
        check_trigger(state, registry, "NAP-queued", t, v);
        if t.source.controller == state.active_player {
            v.push(format!("NAP push queue holds the active player's trigger {} (CR 603.3b)", t.source.description));
        }
    }
    for t in &state.pending_triggers {
        check_trigger(state, registry, "pending", t, v);
        match t.event {
            TriggerEvent::StateTriggered => {}
            TriggerEvent::SelfEntered => {
                if !state.get_object(t.source.id).is_some_and(|o| o.zone == Zone::Battlefield) {
                    v.push(format!("pending enters trigger of #{} whose source is not on the battlefield", t.source.id.0));
                }
            }
            _ => v.push(format!("pending_triggers holds a {:?} trigger; only state and copy-ETB triggers are queued there", t.event)),
        }
    }
    // CR 603.3: every event has been scanned for triggers, and every trigger
    // the state-based actions queued has been bucketed, before anyone is
    // asked anything. The opening-hand loop never runs the collector and a
    // finished game stops before it, so both are outside the claim.
    if !crate::engine::in_mulligan_phase(state) && state.result.is_none() {
        if state.trigger_event_index != state.events.len() {
            v.push(format!("{} of {} events scanned for triggers at a decision point (CR 603.3)",
                state.trigger_event_index, state.events.len()));
        }
        if !state.pending_triggers.is_empty() {
            v.push(format!("{} trigger(s) collected but not bucketed at a decision point (CR 603.3b)",
                state.pending_triggers.len()));
        }
    }
    // A state-triggered ability is on the stack exactly when its source says
    // so, and never twice (CR 603.8).
    let mut state_triggers: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
    let all_triggers = state.stack.iter().filter_map(|e| match e { StackEntry::Trigger(t) => Some(t), _ => None })
        .chain(state.pending_triggers.iter())
        .chain(state.pending_trigger_pushes_ap.iter())
        .chain(state.pending_trigger_pushes_nap.iter());
    for t in all_triggers {
        if matches!(t.event, TriggerEvent::StateTriggered) {
            *state_triggers.entry(t.source.id).or_default() += 1;
        }
    }
    for (id, n) in &state_triggers {
        if *n > 1 {
            v.push(format!("#{} has {n} state-triggered abilities in flight (CR 603.8)", id.0));
        }
    }
    for obj in state.objects_in_id_order() {
        let n = state_triggers.get(&obj.id).copied().unwrap_or(0);
        if obj.state_trigger_on_stack != (n >= 1) {
            v.push(format!("{} (#{}) state_trigger_on_stack={} but {n} such trigger(s) in flight (CR 603.8)",
                obj.name, obj.id.0, obj.state_trigger_on_stack));
        }
    }

    // ── Resolution bookkeeping (CR 608.2m/n, 602.2a, 603.4) ─────────────
    if let Some(id) = state.resolving_spell {
        if state.awaiting_action.is_none() {
            v.push(format!("resolving_spell #{} with no choice pending", id.0));
        }
        match state.get_object(id) {
            None => v.push(format!("resolving_spell #{} does not exist", id.0)),
            Some(o) if o.zone != Zone::Stack => v.push(format!("resolving_spell #{} is in {:?}", id.0, o.zone)),
            _ => {}
        }
        if state.stack.iter().any(|e| e.as_spell() == Some(id)) {
            v.push(format!("resolving_spell #{} is still on the stack", id.0));
        }
        if state.pending_spell_cast.is_some() {
            v.push("a spell is resolving while another is being cast".into());
        }
    }
    if let Some(p) = state.resolving_ability_activator {
        if !player_ok(state, p) {
            v.push(format!("resolving_ability_activator p{} is not a player", p.0));
        }
        if state.awaiting_action.is_none() {
            v.push("resolving_ability_activator set with no choice pending (CR 602.2a scope)".into());
        }
    }
    if state.resolving_trigger_from_back_face.is_some() {
        v.push("resolving_trigger_from_back_face survived past a trigger's hook".into());
    }

    // ── A cast in progress (CR 601.2b/601.2h) ───────────────────────────
    if let Some(c) = &state.pending_spell_cast {
        let what = format!("pending cast of #{}", c.object_id.0);
        if !player_ok(state, c.player) {
            v.push(format!("{what} by p{} who is not a player", c.player.0));
        }
        check_targets(state, &what, &c.targets, v);
        // CR 601.2: the caster holds priority throughout casting.
        if state.priority_player != Some(c.player) {
            v.push(format!("{what} by p{} but priority is {:?} (CR 601.2)", c.player.0, state.priority_player));
        }
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, source, choice }) => {
                let linked = match choice {
                    ResolutionChoiceKind::ChooseXFunding { is_ability: false, source_id, .. } => *source_id == c.object_id,
                    ResolutionChoiceKind::ChooseExileFromGraveyard { source_id, .. } => *source_id == c.object_id,
                    _ => false,
                };
                if !linked || *source != c.object_id || *player != c.player {
                    v.push(format!("{what} but the pending prompt is for #{} / p{} ({:?})", source.0, player.0, std::mem::discriminant(choice)));
                }
            }
            _ => {} // the base check reports the missing prompt
        }
        match state.get_object(c.object_id) {
            None => v.push(format!("{what} names a missing object")),
            Some(o) => {
                if o.card_id != c.card_id {
                    v.push(format!("{what} stashed card {} but the object is card {}", c.card_id.0, o.card_id.0));
                }
                if o.owner != c.player {
                    v.push(format!("{what} by p{} but owned by p{}", c.player.0, o.owner.0));
                }
                if !matches!(o.zone, Zone::Hand | Zone::Graveyard) {
                    v.push(format!("{what} but the spell is in {:?} before its costs are paid", o.zone));
                }
                if c.is_flashback && o.zone != Zone::Graveyard {
                    v.push(format!("{what} with flashback from {:?}", o.zone));
                }
                if o.x_value.is_some() || o.cast_with_flashback {
                    v.push(format!("{what} already carries cast-time marks"));
                }
            }
        }
        let mut sources = std::collections::HashSet::new();
        for (src, _) in &c.tap_plan {
            if !sources.insert(*src) {
                v.push(format!("{what} taps #{} twice", src.0));
            }
            match state.get_object(*src) {
                Some(o) if o.zone == Zone::Battlefield && o.controller == c.player && !o.tapped => {}
                _ => v.push(format!("{what} plans to tap #{} which is not an untapped permanent of the caster", src.0)),
            }
        }
        if let Some(s) = c.sacrifice {
            let ok = s != c.object_id && state.get_object(s).is_some_and(|o|
                o.zone == Zone::Battlefield && o.controller == c.player) && state.is_creature(s, registry);
            if !ok {
                v.push(format!("{what} would sacrifice #{} which is not a creature the caster controls (CR 701.21a)", s.0));
            }
        }
        let mut exiled = std::collections::HashSet::new();
        for e in &c.exile_ids {
            if !exiled.insert(*e) {
                v.push(format!("{what} exiles #{} twice", e.0));
            }
            let ok = *e != c.object_id && state.get_object(*e).is_some_and(|o|
                o.zone == Zone::Graveyard && o.owner == c.player);
            if !ok {
                v.push(format!("{what} would exile #{} which is not in the caster's graveyard", e.0));
            }
        }
    }
    if let Some(a) = &state.pending_ability_effect {
        let what = format!("pending X activation of #{}", a.source_id.0);
        if !player_ok(state, a.activator) {
            v.push(format!("{what} by p{} who is not a player", a.activator.0));
        }
        check_targets(state, &what, &a.targets, v);
        // CR 602.2: the activator holds priority throughout activation.
        if state.priority_player != Some(a.activator) {
            v.push(format!("{what} by p{} but priority is {:?} (CR 602.2)", a.activator.0, state.priority_player));
        }
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice {
                player, source,
                choice: ResolutionChoiceKind::ChooseXFunding { is_ability: true, source_id, .. },
            }) if *source_id == a.source_id && *source == a.source_id && *player == a.activator => {}
            Some(AwaitingAction::ResolutionChoice { .. }) => v.push(format!("{what} but the pending prompt is for something else")),
            _ => {} // the base check reports the missing prompt
        }
        if state.get_object(a.source_id).is_none() {
            v.push(format!("{what} names a missing source"));
        }
        if registry.get(a.behavior_card_id).is_none() {
            v.push(format!("{what} has no behavior in the registry (card {})", a.behavior_card_id.0));
        }
    }
    // The prompt side of the same links.
    if let Some(AwaitingAction::ResolutionChoice { player, source, choice }) = &state.awaiting_action {
        if let ResolutionChoiceKind::ChooseXFunding { options, source_id, .. } = choice {
            // A funding prompt only exists when there is something to fund
            // (an empty ceiling forces X = 0 without asking).
            if options.max_x == 0 {
                v.push(format!("X-funding prompt for #{} with nothing to fund", source_id.0));
            }
            let pooled: u32 = options.pool.values().sum();
            let tappable: u32 = options.groups.iter().map(|g| g.max_contribution()).sum();
            if pooled + tappable != options.max_x {
                v.push(format!("X-funding prompt for #{} offers {pooled} floating + {tappable} tappable but a ceiling of {}",
                    source_id.0, options.max_x));
            }
        }
        match choice {
            ResolutionChoiceKind::ChooseXFunding { is_ability: false, source_id, .. }
            | ResolutionChoiceKind::ChooseExileFromGraveyard { source_id, .. } => {
                if let Some(c) = &state.pending_spell_cast {
                    if c.object_id != *source_id || *source != *source_id || c.player != *player {
                        v.push(format!("cast-time prompt for #{} (p{}) but the stash is for #{} (p{})",
                            source_id.0, player.0, c.object_id.0, c.player.0));
                    }
                }
                if let ResolutionChoiceKind::ChooseExileFromGraveyard { options, min, max, .. } = choice {
                    if min > max {
                        v.push(format!("exile prompt asks for {min}..{max} cards"));
                    }
                    let mut seen = std::collections::HashSet::new();
                    for o in options {
                        if !seen.insert(*o) {
                            v.push(format!("exile prompt offers #{} twice", o.0));
                        }
                        let ok = *o != *source_id && state.get_object(*o).is_some_and(|x|
                            x.zone == Zone::Graveyard && x.owner == *player);
                        if !ok {
                            v.push(format!("exile prompt offers #{} which is not in p{}'s graveyard", o.0, player.0));
                        }
                    }
                }
            }
            ResolutionChoiceKind::ChooseXFunding { is_ability: true, source_id, options, .. } => {
                // The prompt was built from the live pool after the non-X
                // part was paid, and nothing touches the pool until it is
                // answered.
                if player_ok(state, *player) && options.pool != state.get_player(*player).mana_pool.mana {
                    v.push(format!("X-funding prompt for #{} offers pool {:?} but p{} floats {:?}",
                        source_id.0, options.pool, player.0, state.get_player(*player).mana_pool.mana));
                }
                if let Some(a) = &state.pending_ability_effect {
                    if a.source_id != *source_id || a.activator != *player {
                        v.push(format!("X-funding prompt for #{} (p{}) but the stash is for #{} (p{})",
                            source_id.0, player.0, a.source_id.0, a.activator.0));
                    }
                }
            }
            _ => {}
        }
    }
}

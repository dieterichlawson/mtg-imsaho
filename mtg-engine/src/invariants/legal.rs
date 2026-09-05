//! Invariants over the legal action set the engine offers at a decision
//! point (CR 117, 305, 601, 602, 606, 508, 509, 514, 103, 608).
//!
//! `legal_actions` is a pure function of the state; these checks say what
//! that function may offer and what it must, card-independently, so that a
//! menu entry the rules forbid (a sorcery in combat, a tapped attacker, a
//! hexproof target) or a missing one (a land drop lost, the "decline" answer
//! to a "may") is a violation even before anyone picks it.

use super::stack::arity_ok;
use super::{player_ok, Violations};
use crate::actions::{Action, CombatPrompt, ResolvedChoice, Target};
use crate::cards::{CardRegistry, SacrificeCost, TargetRequirement};
use crate::engine::LegalActions;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind, LONDON_MULLIGAN_CAP};
use crate::types::{CardType, ContinuousEffect, CounterType, Keyword, Zone};
use std::collections::{BTreeSet, HashSet};

/// One message per violation of the offer `legal` made to `acting`.
#[must_use]
pub fn check_legal(state: &GameState, acting: PlayerId, legal: &LegalActions, registry: &CardRegistry) -> Violations {
    let mut v = Vec::new();
    if state.result.is_some() || !player_ok(state, acting) {
        return v;
    }
    shape(state, acting, legal, &mut v);
    distinct_offers(legal, &mut v);
    let sorcery = state.step.is_main_phase() && state.stack.is_empty() && state.active_player == acting;
    let stony = state.global_effects(registry).iter().any(|e| matches!(e, ContinuousEffect::PreventArtifactAbilities));
    for a in &legal.actions {
        match a {
            Action::PlayLand { object_id } => play_land(state, acting, *object_id, sorcery, &mut v),
            Action::CastSpell { object_id, targets, sacrifice, tap_plan, alternative_cost, exile_ids, .. } => {
                cast(state, acting, *object_id, targets, *sacrifice, alternative_cost.is_some(), exile_ids, sorcery, registry, &mut v);
                tap_plan_ok(state, acting, tap_plan, Some(*object_id), stony, registry, &mut v);
            }
            Action::ActivateManaAbility { object_id, ability_index } => {
                let ok = state.get_object(*object_id).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == acting)
                    && crate::engine::available_mana_abilities(state, *object_id, registry).iter().any(|m| m.ability_index == *ability_index);
                if !ok {
                    v.push(format!("mana ability {} of #{} offered but not available to p{} (CR 605.3a)", ability_index, object_id.0, acting.0));
                }
                if stony && state.has_card_type(*object_id, CardType::Artifact, registry) {
                    v.push(format!("mana ability of artifact #{} offered under Stony Silence", object_id.0));
                }
            }
            Action::ActivateAbility { object_id, ability_index, targets, tap_plan, sacrifice, x_value, source_card_id } => {
                activate(state, acting, *object_id, *ability_index, targets, *sacrifice, *source_card_id, sorcery, stony, registry, &mut v);
                tap_plan_ok(state, acting, tap_plan, Some(*object_id), stony, registry, &mut v);
                if x_value.is_some() {
                    v.push(format!("ability offer for #{} announces X before funding", object_id.0));
                }
            }
            Action::ActivateLoyaltyAbility { object_id, ability_index, targets } => {
                loyalty(state, acting, *object_id, *ability_index, targets, sorcery, registry, &mut v);
            }
            _ => {}
        }
    }
    for c in &legal.castable_spells {
        tap_plan_ok(state, acting, &c.tap_plan, Some(c.object_id), stony, registry, &mut v);
        if c.is_flashback && state.get_object(c.object_id).is_none_or(|o| o.zone != Zone::Graveyard) {
            v.push(format!("castable spell #{} is marked flashback but is not in the graveyard", c.object_id.0));
        }
    }
    for a in &legal.activatable_abilities {
        tap_plan_ok(state, acting, &a.tap_plan, Some(a.object_id), stony, registry, &mut v);
    }
    collapsed_views(legal, &mut v);
    if let Some(p) = &legal.combat_prompt {
        combat_prompt(state, acting, p, registry, &mut v);
    }
    prompt_offers(state, acting, legal, registry, &mut v);
    v
}

/// CR 117.1/117.3: exactly one way to act, matching the prompt in the state.
fn shape(state: &GameState, acting: PlayerId, legal: &LegalActions, v: &mut Violations) {
    let only = |pred: &dyn Fn(&Action) -> bool, what: &str, v: &mut Violations| {
        if let Some(a) = legal.actions.iter().find(|a| !pred(a)) {
            v.push(format!("{what} offers {a:?}"));
        }
    };
    let no_prompts = |v: &mut Violations, w: &str| {
        if legal.combat_prompt.is_some() || legal.resolution_prompt.is_some() {
            v.push(format!("{w} with a combat or resolution prompt attached"));
        }
    };
    let no_views = |v: &mut Violations, w: &str| {
        if !legal.castable_spells.is_empty() || !legal.activatable_abilities.is_empty() {
            v.push(format!("{w} lists castable spells or activatable abilities"));
        }
    };
    match &state.awaiting_action {
        None => {
            let w = "priority offer";
            if state.priority_player != Some(acting) {
                v.push(format!("{w} to p{} who does not hold priority (CR 117.1)", acting.0));
            }
            no_prompts(v, w);
            if !matches!(legal.actions.first(), Some(Action::PassPriority)) || !matches!(legal.actions.last(), Some(Action::Concede)) {
                v.push(format!("{w} does not start with PassPriority and end with Concede"));
            }
            let passes = legal.actions.iter().filter(|a| matches!(a, Action::PassPriority)).count();
            let concedes = legal.actions.iter().filter(|a| matches!(a, Action::Concede)).count();
            if passes != 1 || concedes != 1 {
                v.push(format!("{w} has {passes} PassPriority and {concedes} Concede entries"));
            }
            only(&|a| matches!(a, Action::PassPriority | Action::Concede | Action::PlayLand { .. } | Action::CastSpell { .. }
                | Action::ActivateManaAbility { .. } | Action::ActivateAbility { .. } | Action::ActivateLoyaltyAbility { .. }), w, v);
        }
        Some(AwaitingAction::DeclareAttackers) => {
            let w = "attackers prompt";
            if acting != state.active_player {
                v.push(format!("{w} offered to p{}, not the active player", acting.0));
            }
            if !matches!(legal.combat_prompt, Some(CombatPrompt::ChooseAttackers { .. })) || legal.resolution_prompt.is_some() {
                v.push(format!("{w} without a ChooseAttackers prompt"));
            }
            if !legal.actions.is_empty() {
                v.push(format!("{w} with {} flat actions", legal.actions.len()));
            }
            no_views(v, w);
        }
        Some(AwaitingAction::DeclareBlockers { defending_player }) => {
            let w = "blockers prompt";
            if acting != *defending_player {
                v.push(format!("{w} offered to p{}, not the defender", acting.0));
            }
            if !matches!(legal.combat_prompt, Some(CombatPrompt::ChooseBlockers { .. })) || legal.resolution_prompt.is_some() {
                v.push(format!("{w} without a ChooseBlockers prompt"));
            }
            if !legal.actions.is_empty() {
                v.push(format!("{w} with {} flat actions", legal.actions.len()));
            }
            no_views(v, w);
        }
        Some(AwaitingAction::DiscardToHandSize { player, .. }) => {
            let w = "discard prompt";
            if acting != *player {
                v.push(format!("{w} offered to p{}, not p{}", acting.0, player.0));
            }
            if legal.actions.is_empty() {
                v.push(format!("{w} with nothing to choose"));
            }
            only(&|a| matches!(a, Action::DiscardCards { .. }), w, v);
            no_prompts(v, w);
            no_views(v, w);
        }
        Some(AwaitingAction::MulliganDecision { player }) => {
            let w = "mulligan prompt";
            if acting != *player {
                v.push(format!("{w} offered to p{}, not p{}", acting.0, player.0));
            }
            only(&|a| matches!(a, Action::MulliganKeep | Action::MulliganMull), w, v);
            no_prompts(v, w);
            no_views(v, w);
        }
        Some(AwaitingAction::BottomAfterMulligan { player, .. }) => {
            let w = "bottoming prompt";
            if acting != *player {
                v.push(format!("{w} offered to p{}, not p{}", acting.0, player.0));
            }
            only(&|a| matches!(a, Action::BottomCards { .. }), w, v);
            no_prompts(v, w);
            no_views(v, w);
        }
        Some(AwaitingAction::ResolutionChoice { player, choice, .. }) => {
            let w = "resolution prompt";
            if acting != *player {
                v.push(format!("{w} offered to p{}, not p{}", acting.0, player.0));
            }
            match &legal.resolution_prompt {
                Some(k) if std::mem::discriminant(k) == std::mem::discriminant(choice) => {}
                other => v.push(format!("{w} carries {:?}, not the pending choice", other.as_ref().map(std::mem::discriminant))),
            }
            if legal.combat_prompt.is_some() {
                v.push(format!("{w} with a combat prompt attached"));
            }
            only(&|a| matches!(a, Action::ResolveChoice { .. }), w, v);
            no_views(v, w);
        }
    }
}

/// A menu, not a multiset.
fn distinct_offers(legal: &LegalActions, v: &mut Violations) {
    let mut seen = HashSet::new();
    for a in &legal.actions {
        if !seen.insert(format!("{a:?}")) {
            v.push(format!("offered twice: {a:?}"));
        }
    }
    let mut keys = HashSet::new();
    for c in &legal.castable_spells {
        if !keys.insert((c.object_id, format!("{:?}", c.alternative_cost))) {
            v.push(format!("castable spell #{} listed twice", c.object_id.0));
        }
    }
    let mut keys = HashSet::new();
    for a in &legal.activatable_abilities {
        if !keys.insert((a.object_id, a.source_card_id, a.ability_index)) {
            v.push(format!("activatable ability #{}/{} listed twice", a.object_id.0, a.ability_index));
        }
    }
}

/// CR 305.1/305.2.
fn play_land(state: &GameState, acting: PlayerId, id: ObjectId, sorcery: bool, v: &mut Violations) {
    let ok = state.get_object(id).is_some_and(|o| o.zone == Zone::Hand && o.owner == acting);
    if !ok {
        v.push(format!("PlayLand #{} which is not in p{}'s hand (CR 305.1)", id.0, acting.0));
    }
    if !sorcery {
        v.push(format!("PlayLand #{} outside a main phase with an empty stack on p{}'s turn (CR 305.1)", id.0, acting.0));
    }
    if state.get_player(acting).land_plays_remaining == 0 {
        v.push(format!("PlayLand #{} with no land drop left (CR 305.2)", id.0));
    }
}

/// `spell`: a spell can't target itself (CR 115.5); an ability may target
/// its source.
#[allow(clippy::too_many_arguments)]
/// CR 601.2c: a card's own restriction on what it may target ("target
/// non-Human creature") lives in its behaviour, not in the shared target
/// requirement, and the shared legality re-check never consults it. An
/// enumerator that drops the predicate offers targets the card forbids, and
/// nothing else here would notice.
fn card_predicate_ok(state: &GameState, acting: PlayerId, card: crate::ids::CardId, targets: &[Target],
                     what: &str, registry: &CardRegistry, v: &mut Violations) {
    let Some(behavior) = registry.get(card) else { return };
    for t in targets {
        if !behavior.is_valid_target(state, acting, t, registry) {
            v.push(format!("{what} offers {t:?} which the card's own restriction rejects (CR 601.2c)"));
        }
    }
}

fn targets_ok(state: &GameState, acting: PlayerId, source: ObjectId, spell: bool, req: &TargetRequirement, targets: &[Target],
              what: &str, registry: &CardRegistry, v: &mut Violations) {
    for (i, t) in targets.iter().enumerate() {
        if targets[..i].contains(t) {
            v.push(format!("{what} targets {t:?} twice"));
        }
        match t {
            Target::Illegal => v.push(format!("{what} offers an Illegal target")),
            Target::Object(id) => {
                if spell && *id == source {
                    v.push(format!("{what} targets itself (CR 115.5)"));
                }
                if !crate::stack::is_target_legal(state, t, req, acting, Some(source), registry) {
                    v.push(format!("{what} offers #{} which is not a legal target now (CR 601.2c)", id.0));
                } else if !crate::engine::can_be_targeted_by(state, *id, acting, Some(source), registry) {
                    v.push(format!("{what} offers #{} which hexproof or protection shields (CR 702.11/702.16)", id.0));
                }
            }
            Target::Player(p) => {
                if !crate::engine::can_target_player(state, *p, acting, registry) {
                    v.push(format!("{what} targets p{} who cannot be targeted", p.0));
                }
            }
        }
    }
}

/// CR 601.1/601.3, 305.9, 702.33a, 307.1.
#[allow(clippy::too_many_arguments)]
fn cast(state: &GameState, acting: PlayerId, id: ObjectId, targets: &[Target], sacrifice: Option<ObjectId>, alt: bool,
        exile_ids: &[ObjectId], sorcery: bool, registry: &CardRegistry, v: &mut Violations) {
    let what = format!("CastSpell #{}", id.0);
    let Some(obj) = state.get_object(id) else {
        v.push(format!("{what} names a missing object"));
        return;
    };
    if obj.owner != acting || !matches!(obj.zone, Zone::Hand | Zone::Graveyard) {
        v.push(format!("{what} from {:?} owned by p{} offered to p{} (CR 601.3a)", obj.zone, obj.owner.0, acting.0));
    }
    let Some(face) = state.face_data(id, registry) else { return };
    if face.card_types.contains(&CardType::Land) {
        v.push(format!("{what} is a land (CR 305.9)"));
    }
    if obj.zone == Zone::Graveyard {
        let permitted = face.flashback_cost.is_some()
            || state.until_end_of_turn.iter().any(|e| matches!(e, crate::state::TemporaryEffect::GrantFlashback { target, .. } if *target == id))
            || registry.get(obj.card_id).is_some_and(|b| b.can_cast_from_graveyard());
        if !permitted {
            v.push(format!("{what} from the graveyard with no permission (CR 601.3a)"));
        }
    }
    let instant_speed = face.card_types.contains(&CardType::Instant) || face.keywords.contains(&Keyword::Flash);
    if !instant_speed && !sorcery {
        v.push(format!("{what} at sorcery speed outside p{}'s main phase with an empty stack (CR 307.1)", acting.0));
    }
    if let Some(ContinuousEffect::PreventCastingNamed { name }) = state.global_effects(registry).iter()
        .find(|e| matches!(e, ContinuousEffect::PreventCastingNamed { name } if *name == face.name))
    {
        v.push(format!("{what} ({name}) is offered while casting it is forbidden"));
    }
    let _ = alt;
    if let Some(s) = sacrifice {
        let ok = state.get_object(s).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == acting) && state.is_creature(s, registry);
        if !ok {
            v.push(format!("{what} would sacrifice #{} which is not a creature p{} controls (CR 701.17a)", s.0, acting.0));
        }
    }
    for e in exile_ids {
        let ok = *e != id && state.get_object(*e).is_some_and(|o| o.zone == Zone::Graveyard && o.owner == acting);
        if !ok {
            v.push(format!("{what} would exile #{} which is not in p{}'s graveyard", e.0, acting.0));
        }
    }
    if let Some(b) = registry.get(obj.card_id) {
        let req = b.target_requirement();
        if !arity_ok(&req, targets.len()) {
            v.push(format!("{what} offers {} targets for {req:?}", targets.len()));
        }
        targets_ok(state, acting, id, true, &req, targets, &what, registry, v);
        card_predicate_ok(state, acting, obj.card_id, targets, &what, registry, v);
    }
    for t in targets {
        if let Target::Object(tid) = t {
            if state.get_object(*tid).is_some_and(|o| o.zone == Zone::Stack) && !state.stack.iter().any(|e| e.as_spell() == Some(*tid)) {
                v.push(format!("{what} targets #{} in the stack zone that is on no stack entry", tid.0));
            }
        }
    }
}

/// CR 601.2g/602.2g, 605.3a: a tap plan taps real, untapped, own sources once each.
fn tap_plan_ok(state: &GameState, acting: PlayerId, plan: &[(ObjectId, usize)], source: Option<ObjectId>, stony: bool,
               registry: &CardRegistry, v: &mut Violations) {
    let mut seen = HashSet::new();
    for (src, idx) in plan {
        if !seen.insert(*src) {
            v.push(format!("tap plan taps #{} twice (CR 602.2h)", src.0));
        }
        // Untapped is not asserted here: it is what
        // `available_mana_abilities` already answers for an ability that
        // taps, and an ability that does not tap needs no such thing.
        let ok = state.get_object(*src).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == acting)
            && crate::engine::available_mana_abilities(state, *src, registry).iter().any(|m| m.ability_index == *idx);
        if !ok {
            v.push(format!("tap plan for #{} taps #{} which is not an available untapped source of p{}", source.map_or(0, |s| s.0), src.0, acting.0));
        }
        if stony && state.has_card_type(*src, CardType::Artifact, registry) {
            v.push(format!("tap plan taps artifact #{} under Stony Silence", src.0));
        }
    }
}

/// CR 602.2, 113.3b, 706.2, 602.5, 302.6, 701.17.
#[allow(clippy::too_many_arguments)]
fn activate(state: &GameState, acting: PlayerId, id: ObjectId, index: usize, targets: &[Target], sacrifice: Option<ObjectId>,
            source_card: Option<crate::ids::CardId>, sorcery: bool, stony: bool, registry: &CardRegistry, v: &mut Violations) {
    let what = format!("ActivateAbility #{}/{}", id.0, index);
    let Some(obj) = state.get_object(id) else {
        v.push(format!("{what} names a missing object"));
        return;
    };
    if obj.zone != Zone::Battlefield || obj.controller != acting {
        v.push(format!("{what} on {:?} controlled by p{} offered to p{} (CR 602.2)", obj.zone, obj.controller.0, acting.0));
    }
    if stony && state.has_card_type(id, CardType::Artifact, registry) {
        v.push(format!("{what} on an artifact under Stony Silence"));
    }
    let def_card = match source_card {
        None => obj.card_id,
        Some(cid) => {
            let via_copy = obj.copy_grantor == Some(cid) && registry.get(cid).is_some_and(|b| b.grants_abilities_to_copies());
            let via_attachment = state.objects_in_id_order().iter().any(|a|
                a.zone == Zone::Battlefield && a.attached_to == Some(id) && a.controller == acting && a.card_id == cid);
            if !via_copy && !via_attachment {
                v.push(format!("{what} through card {} which neither grants it as a copy nor is attached under p{}", cid.0, acting.0));
            }
            cid
        }
    };
    let Some(def) = registry.get(def_card).and_then(|b| b.activated_abilities(state, id, registry).into_iter().find(|d| d.ability_index == index)) else {
        v.push(format!("{what} which card {} does not have", def_card.0));
        return;
    };
    if def.requires_tap && !state.can_pay_tap_cost(id, registry) {
        v.push(format!("{what} needs {{T}} but the permanent cannot tap (CR 302.6)"));
    }
    if let Some((ct, n)) = def.counter_cost {
        if state.get_counter_count(id, ct) < n {
            v.push(format!("{what} needs {n} {ct:?} counters it does not have"));
        }
    }
    if def.once_per_turn && obj.abilities_activated_this_turn.contains(&index) {
        v.push(format!("{what} is once per turn and already used (CR 602.5)"));
    }
    if def.sorcery_speed_only && !sorcery {
        v.push(format!("{what} activates only as a sorcery"));
    }
    match def.sacrifice_cost {
        SacrificeCost::None | SacrificeCost::SacrificeThis => {
            if sacrifice.is_some() {
                v.push(format!("{what} names a sacrifice its cost does not ask for"));
            }
        }
        SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature => match sacrifice {
            None => v.push(format!("{what} names no creature to sacrifice (CR 701.17a)")),
            Some(s) => {
                let ok = state.get_object(s).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == acting) && state.is_creature(s, registry);
                if !ok || (matches!(def.sacrifice_cost, SacrificeCost::SacrificeAnotherCreature) && s == id) {
                    v.push(format!("{what} would sacrifice #{} which is not an eligible creature of p{} (CR 701.17a)", s.0, acting.0));
                }
            }
        },
    }
    match &def.target_requirement {
        None => {
            if !targets.is_empty() {
                v.push(format!("{what} carries targets for an untargeted ability"));
            }
        }
        Some(req) => {
            if targets.len() != 1 {
                v.push(format!("{what} offers {} targets for one requirement", targets.len()));
            }
            targets_ok(state, acting, id, false, req, targets, &what, registry, v);
            card_predicate_ok(state, acting, def_card, targets, &what, registry, v);
        }
    }
}

/// CR 606.3, 606.5, 118.3.
fn loyalty(state: &GameState, acting: PlayerId, id: ObjectId, index: usize, targets: &[Target], sorcery: bool,
           registry: &CardRegistry, v: &mut Violations) {
    let what = format!("ActivateLoyaltyAbility #{}/{}", id.0, index);
    let Some(obj) = state.get_object(id) else {
        v.push(format!("{what} names a missing object"));
        return;
    };
    if obj.zone != Zone::Battlefield || obj.controller != acting || !state.has_card_type(id, CardType::Planeswalker, registry) {
        v.push(format!("{what} is not p{}'s planeswalker on the battlefield (CR 606.3)", acting.0));
    }
    if !sorcery {
        v.push(format!("{what} outside p{}'s main phase with an empty stack (CR 606.3)", acting.0));
    }
    if obj.abilities_activated_this_turn.contains(&999) {
        v.push(format!("{what} after a loyalty ability was used this turn (CR 606.3)"));
    }
    let Some(def) = registry.get(obj.card_id).and_then(|b| b.loyalty_abilities(state, id).into_iter().find(|d| d.ability_index == index)) else {
        v.push(format!("{what} which the card does not have"));
        return;
    };
    if def.loyalty_change < 0 && def.loyalty_change.unsigned_abs() > state.get_counter_count(id, CounterType::Loyalty) {
        v.push(format!("{what} costs {} loyalty of {} (CR 118.3)", -def.loyalty_change, state.get_counter_count(id, CounterType::Loyalty)));
    }
    match &def.target_requirement {
        None => {
            if !targets.is_empty() {
                v.push(format!("{what} carries targets for an untargeted ability"));
            }
        }
        Some(req) => {
            if targets.len() != 1 {
                v.push(format!("{what} offers {} targets for one requirement", targets.len()));
            }
            targets_ok(state, acting, id, false, req, targets, &what, registry, v);
            card_predicate_ok(state, acting, obj.card_id, targets, &what, registry, v);
        }
    }
}

fn distinct_ids(ids: &[ObjectId], what: &str, v: &mut Violations) {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(*id) {
            v.push(format!("{what} lists #{} twice", id.0));
        }
    }
}

/// CR 508.1, 509.1: the combat prompts are exactly the eligible sets.
fn combat_prompt(state: &GameState, acting: PlayerId, prompt: &CombatPrompt, registry: &CardRegistry, v: &mut Violations) {
    match prompt {
        CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player, defending_planeswalkers } => {
            let w = "attackers prompt";
            distinct_ids(eligible, w, v);
            if *defending_player != state.opponent(state.active_player) {
                v.push(format!("{w} names p{} as defender (CR 506.2)", defending_player.0));
            }
            let expected: BTreeSet<ObjectId> = state.objects_in_id_order().iter()
                .filter(|o| o.zone == Zone::Battlefield && o.controller == acting && state.is_creature(o.id, registry)
                    && !o.tapped && (!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))
                    && !state.has_keyword(o.id, Keyword::Defender, registry) && state.can_attack(o.id, registry))
                .map(|o| o.id).collect();
            let offered: BTreeSet<ObjectId> = eligible.iter().copied().collect();
            if offered != expected {
                v.push(format!("{w} offers {offered:?} but the creatures able to attack are {expected:?} (CR 508.1a)"));
            }
            let forced: BTreeSet<ObjectId> = must_attack.iter().copied().collect();
            let expected_forced: BTreeSet<ObjectId> = expected.iter().copied().filter(|id| state.must_attack(*id, registry)).collect();
            if forced != expected_forced {
                v.push(format!("{w} forces {forced:?} but the requirements say {expected_forced:?} (CR 508.1d)"));
            }
            let walkers: BTreeSet<ObjectId> = defending_planeswalkers.iter().copied().collect();
            let expected_walkers: BTreeSet<ObjectId> = state.objects_in_id_order().iter()
                .filter(|o| o.zone == Zone::Battlefield && o.controller == *defending_player && state.has_card_type(o.id, CardType::Planeswalker, registry))
                .map(|o| o.id).collect();
            if walkers != expected_walkers {
                v.push(format!("{w} offers planeswalkers {walkers:?} but the defender has {expected_walkers:?}"));
            }
        }
        CombatPrompt::ChooseBlockers { eligible_blockers, attackers, legal_blocks, min_blockers, .. } => {
            let w = "blockers prompt";
            distinct_ids(eligible_blockers, w, v);
            distinct_ids(attackers, w, v);
            let Some(c) = &state.combat else {
                v.push(format!("{w} with no combat"));
                return;
            };
            let listed: BTreeSet<ObjectId> = attackers.iter().copied().collect();
            let real: BTreeSet<ObjectId> = c.attackers.keys().copied().collect();
            if listed != real {
                v.push(format!("{w} lists attackers {listed:?} but combat has {real:?}"));
            }
            let expected: BTreeSet<ObjectId> = state.objects_in_id_order().iter()
                .filter(|o| o.zone == Zone::Battlefield && o.controller == acting && state.is_creature(o.id, registry)
                    && !o.tapped && state.can_block(o.id, registry))
                .map(|o| o.id).collect();
            let offered: BTreeSet<ObjectId> = eligible_blockers.iter().copied().collect();
            if offered != expected {
                v.push(format!("{w} offers {offered:?} but the creatures able to block are {expected:?} (CR 509.1a)"));
            }
            let keys: BTreeSet<ObjectId> = legal_blocks.keys().copied().collect();
            if keys != offered {
                v.push(format!("{w} has block lists for {keys:?}, not for the eligible blockers"));
            }
            for (b, list) in legal_blocks {
                distinct_ids(list, w, v);
                for a in list {
                    if !real.contains(a) {
                        v.push(format!("{w} lets #{} block #{} which is not attacking", b.0, a.0));
                    }
                }
                for a in &real {
                    let evaded = (state.has_keyword(*a, Keyword::Flying, registry)
                            && !state.has_keyword(*b, Keyword::Flying, registry) && !state.has_keyword(*b, Keyword::Reach, registry))
                        || (state.has_keyword(*a, Keyword::Intimidate, registry) && !state.has_card_type(*b, CardType::Artifact, registry)
                            && state.colors_of(*a, registry).iter().all(|col| !state.colors_of(*b, registry).contains(col)))
                        || state.has_protection_from(*a, *b, registry)
                        || state.cant_be_blocked(*a, registry);
                    let allowed = list.contains(a);
                    if evaded && allowed {
                        v.push(format!("{w} lets #{} block #{} which evades it (CR 509.1b)", b.0, a.0));
                    }
                    let restricted = state.has_effect(*a, &|e| matches!(e, ContinuousEffect::CanOnlyBeBlockedBy { .. }), registry);
                    if !evaded && !allowed && !restricted {
                        v.push(format!("{w} forbids #{} blocking #{} with nothing in the way (CR 509.1a)", b.0, a.0));
                    }
                }
            }
            for (a, n) in min_blockers {
                if !real.contains(a) || *n < 2 {
                    v.push(format!("{w} requires {n} blockers for #{}", a.0));
                }
            }
            for a in &real {
                if state.has_keyword(*a, Keyword::Menace, registry) && min_blockers.get(a).copied().unwrap_or(1) < 2 {
                    v.push(format!("{w} lets a single creature block #{} which has menace (CR 702.111)", a.0));
                }
            }
        }
    }
}

/// CR 514.1, 103.5, 608.2: the prompt's answers are exactly the enumeration
/// of what the state asks.
fn prompt_offers(state: &GameState, acting: PlayerId, legal: &LegalActions, registry: &CardRegistry, v: &mut Violations) {
    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, acting).iter().map(|o| o.id).collect();
    match &state.awaiting_action {
        Some(AwaitingAction::DiscardToHandSize { discard_count, .. }) => {
            let k = (*discard_count).min(hand.len());
            let mut sets = HashSet::new();
            for a in &legal.actions {
                if let Action::DiscardCards { cards } = a {
                    distinct_ids(cards, "discard offer", v);
                    if cards.len() != k || cards.iter().any(|c| !hand.contains(c)) {
                        v.push(format!("discard offer {cards:?} is not {k} cards of p{}'s hand (CR 514.1)", acting.0));
                    }
                    let set: BTreeSet<ObjectId> = cards.iter().copied().collect();
                    if !sets.insert(set) {
                        v.push(format!("discard offer {cards:?} repeats a set"));
                    }
                }
            }
            if hand.len() <= 14 && sets.len() != choose(hand.len(), k) {
                v.push(format!("{} discard offers for {} cards of {} (CR 514.1)", sets.len(), k, hand.len()));
            }
        }
        Some(AwaitingAction::MulliganDecision { .. }) => {
            let capped = state.get_player(acting).mulligan_count >= LONDON_MULLIGAN_CAP;
            let keeps = legal.actions.iter().filter(|a| matches!(a, Action::MulliganKeep)).count();
            let mulls = legal.actions.iter().filter(|a| matches!(a, Action::MulliganMull)).count();
            if keeps != 1 || mulls != usize::from(!capped) {
                v.push(format!("mulligan offer has {keeps} keep and {mulls} mulligan entries at {} mulligans (CR 103.5)",
                    state.get_player(acting).mulligan_count));
            }
        }
        Some(AwaitingAction::BottomAfterMulligan { count, .. }) => {
            let mut sets = HashSet::new();
            for a in &legal.actions {
                if let Action::BottomCards { cards } = a {
                    distinct_ids(cards, "bottom offer", v);
                    if cards.len() != *count || cards.iter().any(|c| !hand.contains(c)) {
                        v.push(format!("bottom offer {cards:?} is not {count} cards of p{}'s hand (CR 103.5)", acting.0));
                    }
                    let set: BTreeSet<ObjectId> = cards.iter().copied().collect();
                    sets.insert(set);
                }
            }
            if *count <= hand.len() && sets.len() != choose(hand.len(), *count) {
                v.push(format!("{} bottom offers for {} cards of {} (CR 103.5)", sets.len(), count, hand.len()));
            }
        }
        Some(AwaitingAction::ResolutionChoice { choice, .. }) => {
            let acts: Vec<&ResolvedChoice> = legal.actions.iter().filter_map(|a| match a {
                Action::ResolveChoice { choice } => Some(choice),
                _ => None,
            }).collect();
            let expect = |v: &mut Violations, want: Vec<String>| {
                let got: Vec<String> = acts.iter().map(|c| format!("{c:?}")).collect();
                if got != want {
                    v.push(format!("resolution offers {got:?} but the prompt enumerates to {want:?} (CR 608.2)"));
                }
            };
            use ResolutionChoiceKind as K;
            match choice {
                K::PayOrNot { cost, .. } => {
                    let mut want = Vec::new();
                    if crate::engine::can_pay_with_sources(state, acting, cost, registry) {
                        want.push(format!("{:?}", ResolvedChoice::PayDecision(true)));
                    }
                    want.push(format!("{:?}", ResolvedChoice::PayDecision(false)));
                    expect(v, want);
                }
                K::ChooseTarget { options, optional, .. } => {
                    let mut want: Vec<String> = options.iter().map(|t| format!("{:?}", ResolvedChoice::ChosenTarget(Some(t.clone())))).collect();
                    if *optional {
                        want.push(format!("{:?}", ResolvedChoice::ChosenTarget(None)));
                    }
                    expect(v, want);
                }
                K::YesNo { .. } => expect(v, vec![format!("{:?}", ResolvedChoice::YesNoDecision(true)), format!("{:?}", ResolvedChoice::YesNoDecision(false))]),
                K::ChooseCardFromHand { cards, .. } => expect(v, cards.iter().map(|c| format!("{:?}", ResolvedChoice::ChosenCard(*c))).collect()),
                K::ChooseFromLookedAt { looked_at, .. } => expect(v, looked_at.iter().map(|c| format!("{:?}", ResolvedChoice::ChosenCard(*c))).collect()),
                K::ChooseFromLibrary { options, .. } => {
                    let mut want: Vec<String> = options.iter().map(|c| format!("{:?}", ResolvedChoice::ChosenCard(*c))).collect();
                    want.push(format!("{:?}", ResolvedChoice::ChosenTarget(None)));
                    expect(v, want);
                }
                K::ChooseCardType { options, .. } | K::ChooseCardName { options, .. } | K::ChooseTriggerOrder { options, .. } => {
                    expect(v, options.iter().enumerate().map(|(i, n)| format!("{:?}", ResolvedChoice::ChosenIndex(i, n.clone()))).collect());
                }
                K::ChoosePile { .. } => {
                    let ok = acts.len() == 2 && matches!(acts[0], ResolvedChoice::ChosenIndex(0, _)) && matches!(acts[1], ResolvedChoice::ChosenIndex(1, _));
                    if !ok {
                        v.push(format!("pile choice offers {} answers, not the two piles", acts.len()));
                    }
                }
                K::DividePermanentsIntoPiles { .. } | K::ChooseXFunding { .. } | K::ChooseExileFromGraveyard { .. } => {
                    if !acts.is_empty() {
                        v.push(format!("a structured prompt offers {} flat answers", acts.len()));
                    }
                }
            }
            // CR 601.2b/605.3a: X is funded from the acting player's real,
            // untapped sources and the real pool.
            if let Some(K::ChooseXFunding { options, source_id, is_ability, .. }) = &legal.resolution_prompt {
                let mut seen = HashSet::new();
                for g in &options.groups {
                    for id in &g.source_ids {
                        if !seen.insert(*id) {
                            v.push(format!("X-funding offers #{} in two groups", id.0));
                        }
                        let abilities = crate::engine::available_mana_abilities(state, *id, registry);
                        let ok = state.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield && o.controller == acting)
                            && abilities.iter().any(|m| m.produced.iter().map(|(_, n)| *n).sum::<u32>() == g.mana_per_tap);
                        if !ok {
                            v.push(format!("X-funding offers #{} which is not an untapped source of p{} producing {}", id.0, acting.0, g.mana_per_tap));
                        }
                    }
                }
                if *is_ability {
                    if state.pending_ability_effect.as_ref().map(|p| p.source_id) != Some(*source_id) {
                        v.push(format!("X-funding for ability of #{} with no matching stash", source_id.0));
                    }
                } else if state.pending_spell_cast.as_ref().map(|c| c.object_id) != Some(*source_id) {
                    v.push(format!("X-funding for spell #{} with no matching stash", source_id.0));
                }
            }
        }
        _ => {}
    }
    // Every object an offer names exists (the cheap catch-all).
    let mut ids: Vec<ObjectId> = Vec::new();
    for a in &legal.actions {
        match a {
            Action::PlayLand { object_id } | Action::ActivateManaAbility { object_id, .. } => ids.push(*object_id),
            Action::CastSpell { object_id, targets, sacrifice, exile_ids, tap_plan, .. } => {
                ids.push(*object_id);
                ids.extend(sacrifice.iter().copied());
                ids.extend(exile_ids.iter().copied());
                ids.extend(tap_plan.iter().map(|(s, _)| *s));
                ids.extend(targets.iter().filter_map(|t| match t { Target::Object(o) => Some(*o), _ => None }));
            }
            Action::ActivateAbility { object_id, targets, sacrifice, tap_plan, .. } => {
                ids.push(*object_id);
                ids.extend(sacrifice.iter().copied());
                ids.extend(tap_plan.iter().map(|(s, _)| *s));
                ids.extend(targets.iter().filter_map(|t| match t { Target::Object(o) => Some(*o), _ => None }));
            }
            Action::ActivateLoyaltyAbility { object_id, targets, .. } => {
                ids.push(*object_id);
                ids.extend(targets.iter().filter_map(|t| match t { Target::Object(o) => Some(*o), _ => None }));
            }
            Action::DiscardCards { cards } | Action::BottomCards { cards } => ids.extend(cards.iter().copied()),
            _ => {}
        }
    }
    for id in ids {
        if state.get_object(id).is_none() {
            v.push(format!("an offer names #{} which does not exist", id.0));
        }
    }
}

fn choose(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    (0..k).fold(1usize, |acc, i| acc * (n - i) / (i + 1))
}

/// The interactive and LLM players act through the collapsed views rather
/// than the flat list, so the two must offer the same game.
fn collapsed_views(legal: &LegalActions, v: &mut Violations) {
    let mut cast_keys: BTreeSet<(u64, String)> = BTreeSet::new();
    let mut ability_groups: std::collections::BTreeMap<(u64, Option<u32>, usize), Vec<(Vec<String>, Option<ObjectId>)>> = std::collections::BTreeMap::new();
    for a in &legal.actions {
        match a {
            Action::CastSpell { object_id, alternative_cost, .. } => {
                cast_keys.insert((object_id.0, format!("{alternative_cost:?}")));
            }
            Action::ActivateAbility { object_id, ability_index, targets, sacrifice, source_card_id, .. } => {
                ability_groups.entry((object_id.0, source_card_id.map(|c| c.0), *ability_index))
                    .or_default()
                    .push((targets.iter().map(|t| format!("{t:?}")).collect(), *sacrifice));
            }
            _ => {}
        }
    }
    let listed_casts: BTreeSet<(u64, String)> = legal.castable_spells.iter()
        .map(|c| (c.object_id.0, format!("{:?}", c.alternative_cost))).collect();
    if listed_casts != cast_keys {
        v.push(format!("castable spells {listed_casts:?} do not match the cast actions {cast_keys:?}"));
    }
    let listed_abilities: BTreeSet<(u64, Option<u32>, usize)> = legal.activatable_abilities.iter()
        .map(|a| (a.object_id.0, a.source_card_id.map(|c| c.0), a.ability_index)).collect();
    let action_abilities: BTreeSet<(u64, Option<u32>, usize)> = ability_groups.keys().copied().collect();
    if listed_abilities != action_abilities {
        v.push(format!("activatable abilities {listed_abilities:?} do not match the activation actions {action_abilities:?}"));
    }
    for entry in &legal.activatable_abilities {
        let key = (entry.object_id.0, entry.source_card_id.map(|c| c.0), entry.ability_index);
        let Some(group) = ability_groups.get(&key) else { continue };
        let combos: Vec<(Vec<String>, Option<ObjectId>)> = entry.option_combos.iter()
            .map(|o| (o.targets.iter().map(|t| format!("{t:?}")).collect(), o.sacrifice)).collect();
        if combos != *group {
            v.push(format!("activatable ability #{}/{} lists {} option(s) for {} action(s)",
                entry.object_id.0, entry.ability_index, combos.len(), group.len()));
        }
        for t in &entry.target_options {
            if !group.iter().any(|(ts, _)| ts.contains(&format!("{t:?}"))) {
                v.push(format!("activatable ability #{}/{} offers target {t:?} that no action uses", entry.object_id.0, entry.ability_index));
            }
        }
    }
}

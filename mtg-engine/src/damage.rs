//! Unified damage pipeline.
//!
//! All damage — combat, fight, and card effects — flows through
//! [`deal_damage`], which applies prevention, replacement, protection,
//! multipliers, planeswalker loyalty removal, deathtouch, `damaged_by`
//! tracking, lifelink, and events in one place. Engine and card code must
//! never write `damage_marked` directly: every hand-rolled copy of this
//! logic has historically missed at least one check (fight damage skipped
//! Unbreathing Horde's prevention; noncombat damage skipped deathtouch
//! and player-lifelink).
//!
//! Damage is queued before it is dealt. Each queued event is *settled*
//! first: every replacement and prevention effect that applies to it
//! (CR 614, 615) is found, and when two or more do, the affected player —
//! the damaged player, or the damaged permanent's controller — chooses
//! which applies first (CR 616.1), after which the rest are asked about
//! again on the event as modified. Only once every queued event is settled
//! is any of it dealt, so a batch that happens at once (a combat damage
//! step, "13 damage to each creature", a fight) is dealt at once, and none
//! of it lands while one of its choices is still open.
//!
//! The pipeline used to run a fixed order — combat prevention, then
//! protection, then Unbreathing Horde, then Inquisitor's Flail, then any
//! card's `replace_event` — and asked nobody. With Inquisitor's Flail and
//! Undead Alchemist both watching one Zombie's combat damage, that order
//! doubled first and milled four where the defending player, who owns the
//! choice, would have milled two (issue #323).

use serde::{Deserialize, Serialize};

use crate::cards::CardRegistry;
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::replacement::{ReplaceableEvent, Replacement};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{ContinuousEffect, CounterType, Keyword, Zone};

/// Whether damage is combat damage. Combat-only modifiers (Ghostly
/// Possession, Moonmist, Inquisitor's Flail, Undead Alchemist's
/// replacement) apply only to `Combat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageKind {
    Combat,
    NonCombat,
}

/// Damage that is about to be dealt.
///
/// It sits in `GameState::pending_damage` from the moment it is queued until
/// it is dealt: through every effect that modifies it, and across the
/// prompt the affected player answers when the order of those effects is
/// theirs to choose (CR 616.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDamage {
    pub source: ObjectId,
    pub target: DamageTarget,
    pub amount: u32,
    pub kind: DamageKind,
    /// Effects that have already modified this event. CR 614.5: an effect
    /// applies to a given event at most once, so these are not offered
    /// again. Only modifications get here — an effect that prevents or
    /// replaces the damage ends the event instead.
    #[serde(default)]
    pub applied: Vec<DamageEffect>,
    /// Every applicable effect has been dealt with; what is left is dealt as
    /// it stands.
    #[serde(default)]
    pub settled: bool,
}

/// One replacement or prevention effect that applies to a damage event —
/// the things the affected player chooses among (CR 616.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageEffect {
    /// A prevention effect that prevents all of the damage and does nothing
    /// else (CR 615).
    PreventAll(Prevention),
    /// "If this creature would be dealt damage, prevent that damage and
    /// remove a +1/+1 counter from it" — Unbreathing Horde, which is `by`.
    PreventAndRemoveCounter { by: ObjectId },
    /// "It deals double that damage instead" — Inquisitor's Flail, which is
    /// `by`, on the source or on the target.
    Double { by: ObjectId },
    /// A card's own replacement effect (CR 614.1b): offered through
    /// `CardBehavior::replacement_offer` and applied through
    /// `CardBehavior::replace_event`. `by` is the permanent whose effect it
    /// is — Undead Alchemist.
    Card { by: ObjectId },
}

/// What is preventing all of a damage event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prevention {
    /// A permanent's static ability — Ghostly Possession on the source or
    /// the target.
    Permanent(ObjectId),
    /// An effect that lasts the turn with no permanent behind it — Moonmist
    /// — named by the card that made it.
    ThisTurn(String),
    /// The target's protection from the source (CR 702.16e).
    Protection,
}

/// Deal damage from a source object to a creature, planeswalker, or player.
/// The single entry point for all damage in the engine.
///
/// One event, queued and dealt at once — unless the affected player has a
/// choice to make about it (CR 616.1), in which case it waits in
/// `pending_damage` for their answer, along with anything queued after it.
/// For a batch that happens simultaneously, see [`queue_damage`].
pub fn deal_damage(
    state: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    amount: u32,
    kind: DamageKind,
    registry: &CardRegistry,
) {
    queue_damage(state, source, target, amount, kind);
    process_pending_damage(state, registry);
}

/// Queue damage without dealing it yet.
///
/// For damage that happens at once — every creature's combat damage
/// (CR 510.2), "13 damage to each creature", the two halves of a fight
/// (CR 701.12a): queue all of it, then call [`process_pending_damage`]
/// once, so that every event's effects are settled before any of it is
/// dealt and none of it is dealt while a choice about another is open.
pub fn queue_damage(
    state: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    amount: u32,
    kind: DamageKind,
) {
    if amount == 0 {
        return;
    }
    state.pending_damage.push(PendingDamage {
        source, target, amount, kind, applied: Vec::new(), settled: false,
    });
}

/// Settle and deal everything in `pending_damage`.
///
/// Settling an event means applying the replacement and prevention effects
/// that apply to it, one at a time, re-reading what applies after each
/// (CR 616.1). When two or more apply and the order changes what happens,
/// the affected player is asked which applies first and this returns with
/// the queue as it stands; the answer comes back through
/// `Action::ResolveChoice`, which applies the chosen effect and calls this
/// again. When every order ends the same way — two Flails, two
/// Alchemists, a Flail on a creature that Ghostly Possession is already
/// shielding — nothing is asked and the effects apply in a fixed order.
///
/// Once every queued event is settled, all of it is dealt, in the order it
/// was queued.
pub fn process_pending_damage(state: &mut GameState, registry: &CardRegistry) {
    let mut i = 0;
    while i < state.pending_damage.len() {
        if effect_choice_is_open(state) {
            return;
        }
        if state.pending_damage[i].settled {
            i += 1;
            continue;
        }
        let pd = state.pending_damage[i].clone();
        if !can_be_dealt_damage(state, &pd.target) {
            // A permanent that has left the battlefield is not dealt damage
            // (CR 120.1); the event simply does not happen.
            state.pending_damage.remove(i);
            continue;
        }
        let effects = applicable_effects(state, &pd, registry);
        if effects.is_empty() {
            state.pending_damage[i].settled = true;
            i += 1;
            continue;
        }
        if effects.len() == 1 || same_outcome_in_every_order(&effects, pd.amount, state) {
            // Listed with the preventions first: the same place by the
            // fewest steps. The event may be gone afterwards, in which case
            // the next one is now at this index.
            apply_effect(state, i, &effects[0], registry);
            continue;
        }
        if state.awaiting_action.is_some() {
            // Some other question is already up. Nothing in the engine
            // deals damage with a prompt open — turn-based actions and
            // resolutions run between prompts, and an answer takes its
            // prompt down before acting on it — so this is reached only by
            // a test driving the damage step over an unanswered prompt.
            // The choice cannot be asked without discarding that one, so it
            // is made in the fixed order rather than lost.
            state.log(LogLevel::Debug, format!(
                "a choice is already open; {} applies first to {} unasked",
                effect_name(state, &effects[0]), describe_event(state, &pd)));
            apply_effect(state, i, &effects[0], registry);
            continue;
        }
        ask_affected_player(state, &pd, effects, registry);
        return;
    }
    if effect_choice_is_open(state) {
        return;
    }
    let ready = std::mem::take(&mut state.pending_damage);
    for pd in ready {
        perform(state, pd, registry);
    }
}

/// Apply the effect the affected player chose for the event a
/// `ChooseDamageEffect` prompt was about, then carry on settling and
/// dealing the queue.
///
/// `chooser` is the player who answered; `event` identifies the pending
/// damage the prompt described. The effect is checked against what applies
/// *now*: the choice was offered before any state-based action ran, and a
/// permanent whose effect was offered may have died in the meantime, in
/// which case the choice is void and the event is settled afresh.
pub fn apply_chosen_effect(
    state: &mut GameState,
    chooser: PlayerId,
    event: &PendingDamage,
    effect: &DamageEffect,
    registry: &CardRegistry,
) {
    let position = state.pending_damage.iter().position(|p| !p.settled);
    let matches = position.is_some_and(|i| {
        let p = &state.pending_damage[i];
        p.source == event.source && p.target == event.target
            && p.amount == event.amount && p.kind == event.kind
    });
    if let (Some(i), true) = (position, matches) {
        let pd = state.pending_damage[i].clone();
        if applicable_effects(state, &pd, registry).contains(effect) {
            let name = effect_name(state, effect);
            state.log(LogLevel::Event, format!(
                "p{}: {name} applies first to {} (CR 616.1)",
                chooser.0, describe_event(state, &pd)));
            apply_effect(state, i, effect, registry);
        } else {
            state.log(LogLevel::Debug, format!(
                "the chosen effect {effect:?} no longer applies to {}; settling it again",
                describe_event(state, &pd)));
        }
    } else {
        state.log(LogLevel::Debug,
            "the damage a choice was made for is no longer waiting".into());
    }
    process_pending_damage(state, registry);
}

/// Whether a `ChooseDamageEffect` prompt is up — the one question the queue
/// waits on.
fn effect_choice_is_open(state: &GameState) -> bool {
    matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseDamageEffect { .. }, .. }))
}

/// The replacement and prevention effects that apply to `pd` as it stands,
/// with the preventions first, then Unbreathing Horde's, then Inquisitor's
/// Flail's, then cards' own; within each, by object id. Effects that have
/// already applied to this event are left out (CR 614.5).
#[must_use]
pub fn applicable_effects(state: &GameState, pd: &PendingDamage, registry: &CardRegistry) -> Vec<DamageEffect> {
    let mut out: Vec<DamageEffect> = Vec::new();
    let combat = pd.kind == DamageKind::Combat;
    let source = pd.source;
    let target_object = match pd.target {
        DamageTarget::Object(id) => Some(id),
        DamageTarget::Player(_) => None,
    };

    if combat {
        // "Prevent all combat damage that would be dealt to and dealt by
        // enchanted creature" (Ghostly Possession), from either end.
        let mut shields: Vec<ObjectId> = Vec::new();
        for id in std::iter::once(source).chain(target_object) {
            state.walk_effects(id,
                &|e| matches!(e, ContinuousEffect::PreventCombatDamage { .. }),
                registry,
                &mut |_, by| { shields.push(by.id); true });
        }
        shields.sort_unstable_by_key(|id| id.0);
        shields.dedup();
        out.extend(shields.into_iter().map(|id| DamageEffect::PreventAll(Prevention::Permanent(id))));

        // "Prevent all combat damage that would be dealt this turn by
        // creatures other than <filter>" (Moonmist). The filter names the
        // exceptions; a source that matches none of them is prevented.
        let controller = state.get_object(source).map_or(PlayerId(0), |o| o.controller);
        let mut turn_wide: Vec<String> = state.until_end_of_turn.iter()
            .filter_map(|e| match e {
                crate::state::TemporaryEffect::PreventCombatDamageExcept { filter, source_name }
                    // No permanent stands behind the effect, so the damage
                    // source stands in as the "source" the filter is read
                    // against, which only `ControlledByAttachedPlayer` would
                    // notice and no such effect uses.
                    if !state.matches_filter(source, filter, source, controller, registry) =>
                        Some(source_name.clone()),
                _ => None,
            })
            .collect();
        turn_wide.sort();
        turn_wide.dedup();
        out.extend(turn_wide.into_iter().map(|name| DamageEffect::PreventAll(Prevention::ThisTurn(name))));
    }

    // Protection from the source prevents the damage (CR 702.16e).
    let protected = match pd.target {
        DamageTarget::Object(t) => state.has_protection_from(t, source, registry),
        DamageTarget::Player(p) => state.colors_of(source, registry).into_iter()
            .any(|c| state.player_has_protection_from(p, c, registry)),
    };
    if protected {
        out.push(DamageEffect::PreventAll(Prevention::Protection));
    }

    // "Prevent that damage and remove a +1/+1 counter" (Unbreathing Horde).
    if let Some(t) = target_object {
        let mut hordes: Vec<ObjectId> = Vec::new();
        state.walk_effects(t,
            &|e| matches!(e, ContinuousEffect::PreventDamageRemoveCounter { .. }),
            registry,
            &mut |_, by| { hordes.push(by.id); true });
        hordes.sort_unstable_by_key(|id| id.0);
        hordes.dedup();
        out.extend(hordes.into_iter().map(|by| DamageEffect::PreventAndRemoveCounter { by }));
    }

    if combat {
        // "It deals double that damage instead" (Inquisitor's Flail): each
        // Flail on the source doubles what it deals, each on the target
        // doubles what it is dealt, and each applies once.
        let mut flails: Vec<ObjectId> = Vec::new();
        for id in std::iter::once(source).chain(target_object) {
            state.walk_effects(id,
                &|e| matches!(e, ContinuousEffect::DoubleCombatDamage { .. }),
                registry,
                &mut |_, by| { flails.push(by.id); true });
        }
        flails.sort_unstable_by_key(|id| id.0);
        flails.dedup();
        out.extend(flails.into_iter().map(|by| DamageEffect::Double { by }));
    }

    // Cards' own replacement effects (CR 614), asked read-only.
    let event = replaceable(pd);
    for o in state.objects_in_id_order() {
        let Some(behavior) = registry.get(o.card_id) else { continue };
        if !behavior.replacement_zones().contains(&o.zone) {
            continue;
        }
        if behavior.replacement_offer(state, o.id, &event, registry).is_some() {
            out.push(DamageEffect::Card { by: o.id });
        }
    }

    out.retain(|e| !pd.applied.contains(e));
    out
}

/// The event as the replacement layer sees it.
fn replaceable(pd: &PendingDamage) -> ReplaceableEvent {
    ReplaceableEvent::DealsDamage {
        source: pd.source,
        target: pd.target,
        amount: pd.amount,
        combat: pd.kind == DamageKind::Combat,
    }
}

/// Whether the damage can be dealt to its target at all: a permanent has to
/// be on the battlefield (CR 120.1); a player always can be.
fn can_be_dealt_damage(state: &GameState, target: &DamageTarget) -> bool {
    match target {
        DamageTarget::Object(id) => state.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield),
        DamageTarget::Player(_) => true,
    }
}

/// The player CR 616.1 gives the choice to: the damaged player, or the
/// damaged permanent's controller.
#[must_use]
pub fn affected_player(state: &GameState, target: &DamageTarget) -> PlayerId {
    match target {
        DamageTarget::Player(p) => *p,
        DamageTarget::Object(id) => state.get_object(*id).map_or(PlayerId(0), |o| o.controller),
    }
}

/// How a run of effects ends, as far as the choice among them is concerned.
///
/// This is the model behind "does the order matter?": the damage is dealt
/// at some amount, or prevented (with or without Unbreathing Horde's
/// counter coming off), or replaced by a card's effect at some amount. A
/// prevention or a card's replacement ends the event, so nothing after it
/// counts; a doubling carries on. Two effects of the same card are one
/// outcome — two Undead Alchemists mill once between them, whichever is
/// first — so a card's outcome is keyed by its card, not its object.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Dealt(u32),
    Prevented { counter_removed: bool },
    Replaced { card: CardId, amount: u32 },
}

fn outcome_of(order: &[&DamageEffect], amount: u32, state: &GameState) -> Outcome {
    let mut amount = amount;
    for effect in order {
        match effect {
            DamageEffect::PreventAll(_) => return Outcome::Prevented { counter_removed: false },
            DamageEffect::PreventAndRemoveCounter { .. } => return Outcome::Prevented { counter_removed: true },
            DamageEffect::Double { .. } => amount = amount.saturating_mul(2),
            DamageEffect::Card { by } => {
                let card = state.get_object(*by).map_or(CardId(0), |o| o.card_id);
                return Outcome::Replaced { card, amount };
            }
        }
    }
    Outcome::Dealt(amount)
}

/// Whether every order of `effects` ends the same way, so that the choice
/// among them is no choice at all and nobody needs to be asked.
///
/// The model is conservative: a card's replacement is taken to end the
/// event, and where that is wrong for some future card the answer can only
/// be "ask" where "don't" would have done, never the reverse. Past seven
/// effects on one event the orders are not enumerated and the player is
/// asked.
fn same_outcome_in_every_order(effects: &[DamageEffect], amount: u32, state: &GameState) -> bool {
    let n = effects.len();
    if n < 2 {
        return true;
    }
    if n > 7 {
        return false;
    }
    let mut index: Vec<usize> = (0..n).collect();
    let first = outcome_of(&index.iter().map(|&k| &effects[k]).collect::<Vec<_>>(), amount, state);
    while next_permutation(&mut index) {
        let order: Vec<&DamageEffect> = index.iter().map(|&k| &effects[k]).collect();
        if outcome_of(&order, amount, state) != first {
            return false;
        }
    }
    true
}

/// Advance `p` to the next permutation in lexicographic order; false once
/// it was the last.
fn next_permutation(p: &mut [usize]) -> bool {
    let n = p.len();
    if n < 2 {
        return false;
    }
    let mut i = n - 1;
    while i > 0 && p[i - 1] >= p[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    let mut j = n - 1;
    while p[j] <= p[i - 1] {
        j -= 1;
    }
    p.swap(i - 1, j);
    p[i..].reverse();
    true
}

/// Put the CR 616.1 choice to the affected player.
fn ask_affected_player(
    state: &mut GameState,
    pd: &PendingDamage,
    effects: Vec<DamageEffect>,
    registry: &CardRegistry,
) {
    let player = affected_player(state, &pd.target);
    let options: Vec<String> = effects.iter()
        .map(|e| effect_label(state, e, pd, player, registry))
        .collect();
    let description = format!(
        "{}: {} effects apply — choose the one to apply first (CR 616.1); the rest \
         apply afterwards if they still can",
        describe_event_for(state, pd, player), effects.len());
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player,
        source: pd.source,
        choice: ResolutionChoiceKind::ChooseDamageEffect {
            description,
            effects,
            options,
            source: pd.source,
            target: pd.target,
            amount: pd.amount,
            kind: pd.kind,
        },
    });
}

/// "Walking Corpse (#30)'s 2 combat damage to p1".
fn describe_event(state: &GameState, pd: &PendingDamage) -> String {
    format!("{}'s {} {}damage to {}",
        state.obj_name(pd.source), pd.amount,
        if pd.kind == DamageKind::Combat { "combat " } else { "" },
        target_name(state, &pd.target))
}

/// The same, addressed to `reader`: "Walking Corpse (#30) would deal 2
/// combat damage to you".
fn describe_event_for(state: &GameState, pd: &PendingDamage, reader: PlayerId) -> String {
    let to = match pd.target {
        DamageTarget::Player(p) if p == reader => "you".to_string(),
        _ => target_name(state, &pd.target),
    };
    format!("{} would deal {} {}damage to {to}",
        state.obj_name(pd.source), pd.amount,
        if pd.kind == DamageKind::Combat { "combat " } else { "" })
}

fn target_name(state: &GameState, target: &DamageTarget) -> String {
    match target {
        DamageTarget::Player(p) => format!("p{}", p.0),
        DamageTarget::Object(id) => state.obj_name(*id),
    }
}

/// What an effect is called in the log: the permanent, the card that made
/// the turn-wide effect, or "protection".
fn effect_name(state: &GameState, effect: &DamageEffect) -> String {
    match effect {
        DamageEffect::PreventAll(Prevention::Permanent(id))
        | DamageEffect::PreventAndRemoveCounter { by: id }
        | DamageEffect::Double { by: id }
        | DamageEffect::Card { by: id } => state.obj_name(*id),
        DamageEffect::PreventAll(Prevention::ThisTurn(name)) => name.clone(),
        DamageEffect::PreventAll(Prevention::Protection) => "protection".into(),
    }
}

/// What an effect would do, as the option the affected player picks.
fn effect_label(
    state: &GameState,
    effect: &DamageEffect,
    pd: &PendingDamage,
    reader: PlayerId,
    registry: &CardRegistry,
) -> String {
    let on = |id: ObjectId| -> String {
        state.get_object(id).and_then(|o| o.attached_to)
            .map_or_else(String::new, |host| format!(" on {}", state.obj_name(host)))
    };
    match effect {
        DamageEffect::PreventAll(Prevention::Permanent(id)) =>
            format!("{}{}: prevent all of it", state.obj_name(*id), on(*id)),
        DamageEffect::PreventAll(Prevention::ThisTurn(name)) =>
            format!("{name}: prevent all of it"),
        DamageEffect::PreventAll(Prevention::Protection) => {
            let whose = match pd.target {
                DamageTarget::Player(p) if p == reader => "your".to_string(),
                _ => format!("{}'s", target_name(state, &pd.target)),
            };
            format!("{whose} protection from {}: prevent all of it", state.obj_name(pd.source))
        }
        DamageEffect::PreventAndRemoveCounter { by } => {
            let left = state.get_counter_count(*by, CounterType::PlusOnePlusOne);
            format!("{}: prevent all of it and remove a +1/+1 counter from it ({left} on it)",
                state.obj_name(*by))
        }
        DamageEffect::Double { by } =>
            format!("{}{}: double it to {}", state.obj_name(*by), on(*by), pd.amount.saturating_mul(2)),
        DamageEffect::Card { by } => {
            let offer = state.get_object(*by)
                .and_then(|o| registry.get(o.card_id))
                .and_then(|b| b.replacement_offer(state, *by, &replaceable(pd), registry));
            offer.unwrap_or_else(|| format!("{}: its replacement effect", state.obj_name(*by)))
        }
    }
}

/// Apply one effect to the pending damage at `index`: a prevention or a
/// card's replacement removes the event from the queue; a modification
/// changes it in place and is recorded as applied (CR 614.5).
fn apply_effect(state: &mut GameState, index: usize, effect: &DamageEffect, registry: &CardRegistry) {
    let pd = state.pending_damage[index].clone();
    let source_name = state.obj_name(pd.source);
    let target = target_name(state, &pd.target);
    let combat = if pd.kind == DamageKind::Combat { "combat " } else { "" };
    let amount = pd.amount;
    match effect {
        DamageEffect::PreventAll(prevention) => {
            // Said out loud — a silent damage step read as the engine
            // forgetting combat, not as prevention working (issue #137) —
            // and with the amount and the source, so that three points off
            // a Brimstone Volley and six off a blocked attacker do not
            // print the same line (issue #299).
            let by = match prevention {
                Prevention::Protection => "protection".to_string(),
                _ => effect_name(state, effect),
            };
            state.log(LogLevel::Event, format!(
                "{target}: {amount} {combat}damage from {source_name} prevented by {by}"));
            state.pending_damage.remove(index);
        }
        DamageEffect::PreventAndRemoveCounter { by } => {
            let counters = state.get_counter_count(*by, CounterType::PlusOnePlusOne);
            if counters > 0 {
                if let Some(obj) = state.get_object_mut(*by) {
                    let entry = obj.counters.entry(CounterType::PlusOnePlusOne).or_insert(0);
                    *entry = entry.saturating_sub(1);
                    if *entry == 0 {
                        obj.counters.remove(&CounterType::PlusOnePlusOne);
                    }
                }
                state.log(LogLevel::Event, format!(
                    "{target}: {amount} damage from {source_name} prevented, removed a +1/+1 counter"));
            } else {
                // Ruling (2011-09-22): the effect still prevents the damage
                // with no counter left to remove.
                state.log(LogLevel::Event, format!(
                    "{target}: {amount} damage from {source_name} prevented (no +1/+1 counter to remove)"));
            }
            state.pending_damage.remove(index);
        }
        DamageEffect::Double { by } => {
            let doubled = amount.saturating_mul(2);
            let flail = state.obj_name(*by);
            state.log(LogLevel::Debug, format!(
                "{flail}: {source_name}'s {amount} combat damage to {target} is doubled to {doubled}"));
            let entry = &mut state.pending_damage[index];
            entry.amount = doubled;
            entry.applied.push(effect.clone());
        }
        DamageEffect::Card { by } => {
            let behavior = state.get_object(*by).and_then(|o| registry.get(o.card_id));
            let outcome = behavior.and_then(|b| b.replace_event(state, *by, &replaceable(&pd), registry));
            match outcome {
                Some(Replacement::Replaced) => {
                    state.pending_damage.remove(index);
                }
                Some(Replacement::Modified(ReplaceableEvent::DealsDamage { source, target, amount, combat })) => {
                    let entry = &mut state.pending_damage[index];
                    entry.source = source;
                    entry.target = target;
                    entry.amount = amount;
                    entry.kind = if combat { DamageKind::Combat } else { DamageKind::NonCombat };
                    entry.applied.push(effect.clone());
                }
                Some(Replacement::Modified(_)) | None => {
                    // The card offered the effect and then did not apply it:
                    // a card bug, and not one to loop on.
                    let name = state.obj_name(*by);
                    state.log(LogLevel::Debug, format!(
                        "{name} offered a replacement for {source_name}'s damage to {target} and did not apply it"));
                    state.pending_damage[index].applied.push(effect.clone());
                }
            }
        }
    }
}

/// Deal settled damage: mark it (or remove loyalty), record who dealt it,
/// emit the event and the log line, and gain lifelink life.
fn perform(state: &mut GameState, pd: PendingDamage, registry: &CardRegistry) {
    let PendingDamage { source, target, amount, kind, .. } = pd;
    if amount == 0 {
        return;
    }
    match target {
        DamageTarget::Object(id) => perform_on_object(state, source, id, amount, kind, registry),
        DamageTarget::Player(pid) => perform_on_player(state, source, pid, amount, kind, registry),
    }
}

fn perform_on_object(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    kind: DamageKind,
    registry: &CardRegistry,
) {
    if state.get_object(target).is_none_or(|o| o.zone != Zone::Battlefield) {
        return;
    }

    let has_deathtouch = state.has_keyword(source, Keyword::Deathtouch, registry);
    let is_planeswalker = state.has_card_type(target, crate::types::CardType::Planeswalker, registry);

    if let Some(obj) = state.get_object_mut(target) {
        if is_planeswalker {
            // Damage to a planeswalker removes that many loyalty counters (CR 120.3c).
            let loyalty = obj.counters.entry(CounterType::Loyalty).or_insert(0);
            *loyalty = loyalty.saturating_sub(amount);
            if *loyalty == 0 {
                obj.counters.remove(&CounterType::Loyalty);
            }
        } else {
            obj.damage_marked += amount;
            if has_deathtouch {
                obj.dealt_deathtouch_damage = true;
            }
        }
        // Track which objects damaged this one (Abattoir Ghoul, Into the Maw of Hell).
        if !obj.damaged_by.contains(&source) {
            obj.damaged_by.push(source);
        }
    }

    let event_target = DamageTarget::Object(target);
    match kind {
        DamageKind::Combat => {
            state.events.push(GameEvent::CombatDamageDealt {
                source, target: event_target, amount,
            });
            // Logged like its non-combat sibling below: without this line a
            // blocker died with no stated cause and the log's combat math
            // could not be reconciled after the fact (issue #89).
            let source_name = state.obj_name(source);
            state.log(LogLevel::Event,
                format!("{} dealt {} combat damage to {}",
                    source_name, amount, state.obj_name(target)));
        }
        DamageKind::NonCombat => {
            state.events.push(GameEvent::NonCombatDamageDealt {
                source, target: event_target, amount,
            });
            let source_name = state.obj_name(source);
            state.log(LogLevel::Event,
                format!("{} dealt {} damage to {}", source_name, amount, state.obj_name(target)));
        }
    }

    apply_lifelink(state, source, amount, registry);
}

fn perform_on_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
    kind: DamageKind,
    registry: &CardRegistry,
) {
    // The life change itself is `change_life`, which emits LifeChanged; the
    // damage events below are what make this damage rather than life loss.
    // Both events used to be pushed here side by side, which meant this was the
    // one place in the codebase where a LifeChanged could be emitted twice for
    // one change if the helper were ever used alongside it.
    // Quiet: the damage lines below carry the change and the total.
    state.change_life_quiet(player, -i32::try_from(amount).unwrap_or(i32::MAX));
    let new_life = state.get_player(player).life;

    match kind {
        DamageKind::Combat => {
            state.events.push(GameEvent::CombatDamageDealt {
                source, target: DamageTarget::Player(player), amount,
            });
            state.log(LogLevel::Event,
                format!("p{} took {} combat damage ({}) from {}", player.0, amount, new_life, state.obj_name(source)));
        }
        DamageKind::NonCombat => {
            state.events.push(GameEvent::NonCombatDamageDealt {
                source, target: DamageTarget::Player(player), amount,
            });
            state.log(LogLevel::Event,
                format!("{} dealt {} damage to p{} ({})", state.obj_name(source), amount, player.0, new_life));
        }
    }

    apply_lifelink(state, source, amount, registry);
}

/// Lifelink: the source's controller gains life equal to the damage dealt
/// (CR 702.15) — combat and noncombat alike.
fn apply_lifelink(state: &mut GameState, source: ObjectId, amount: u32, registry: &CardRegistry) {
    if !state.has_keyword(source, Keyword::Lifelink, registry) {
        return;
    }
    let Some(controller) = state.get_object(source).map(|o| o.controller) else { return };
    // Quiet: the lifelink line below carries the change and the total.
    state.change_life_quiet(controller, i32::try_from(amount).unwrap_or(i32::MAX));
    // Say so, with the running total — an unexplained 6-point life swing made
    // the log's life figures impossible to reconcile (issue #89).
    let new_life = state.get_player(controller).life;
    state.log(LogLevel::Event, format!(
        "{} (lifelink): p{} gained {} life ({})",
        state.obj_name(source), controller.0, amount, new_life));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn possession(id: u64) -> DamageEffect { DamageEffect::PreventAll(Prevention::Permanent(ObjectId(id))) }
    fn flail(id: u64) -> DamageEffect { DamageEffect::Double { by: ObjectId(id) } }
    fn horde(id: u64) -> DamageEffect { DamageEffect::PreventAndRemoveCounter { by: ObjectId(id) } }

    /// A card's replacement needs an object to read the card off; the
    /// abstract model keys it by that card.
    fn state_with_cards(cards: &[(u64, u32)]) -> GameState {
        let mut state = GameState::new(2);
        for &(object, card) in cards {
            let id = state.create_object(CardId(card), PlayerId(0), Zone::Battlefield, None, None);
            // `create_object` numbers objects itself; the test names them by
            // the ids it was handed back.
            assert_eq!(id.0, object, "objects are created in the order the test lists them");
        }
        state
    }

    #[test]
    fn a_flail_and_an_alchemist_are_a_real_choice() {
        let state = state_with_cards(&[(1, 7)]);
        let effects = [flail(40), DamageEffect::Card { by: ObjectId(1) }];
        assert!(!same_outcome_in_every_order(&effects, 2, &state),
            "mill 2 or mill 4 depending on which is first");
    }

    #[test]
    fn a_shield_and_an_alchemist_are_a_real_choice() {
        let state = state_with_cards(&[(1, 7)]);
        let effects = [possession(43), DamageEffect::Card { by: ObjectId(1) }];
        assert!(!same_outcome_in_every_order(&effects, 2, &state),
            "nothing happens, or a mill happens");
    }

    #[test]
    fn a_shield_and_a_flail_are_no_choice() {
        let state = GameState::new(2);
        assert!(same_outcome_in_every_order(&[possession(43), flail(40)], 2, &state),
            "doubled or not, all of it is prevented");
        assert!(same_outcome_in_every_order(&[possession(43), possession(44), flail(40), flail(41)], 2, &state));
    }

    #[test]
    fn a_horde_and_a_flail_are_no_choice_but_a_horde_and_a_shield_are() {
        let state = GameState::new(2);
        assert!(same_outcome_in_every_order(&[horde(12), flail(40)], 3, &state),
            "prevented with a counter removed either way");
        assert!(!same_outcome_in_every_order(&[horde(12), possession(43)], 3, &state),
            "the counter stays if the shield goes first");
    }

    #[test]
    fn two_of_the_same_card_are_no_choice() {
        let state = state_with_cards(&[(1, 7), (2, 7)]);
        let two = [DamageEffect::Card { by: ObjectId(1) }, DamageEffect::Card { by: ObjectId(2) }];
        assert!(same_outcome_in_every_order(&two, 2, &state), "two Alchemists mill once");
        let state = state_with_cards(&[(1, 7), (2, 8)]);
        let different = [DamageEffect::Card { by: ObjectId(1) }, DamageEffect::Card { by: ObjectId(2) }];
        assert!(!same_outcome_in_every_order(&different, 2, &state), "two different cards are a choice");
    }

    #[test]
    fn two_flails_are_no_choice() {
        let state = GameState::new(2);
        assert!(same_outcome_in_every_order(&[flail(40), flail(41)], 2, &state));
    }

    #[test]
    fn permutations_are_all_visited_once() {
        let mut p = vec![0, 1, 2];
        let mut seen = vec![p.clone()];
        while next_permutation(&mut p) {
            assert!(!seen.contains(&p), "{p:?} twice");
            seen.push(p.clone());
        }
        assert_eq!(seen.len(), 6);
        let mut one = vec![0];
        assert!(!next_permutation(&mut one));
    }
}

//! Shared helper functions for common card resolution patterns.
//!
//! Includes:
//! - Spell resolution helpers (`resolve_aura`, `resolve_damage`, `resolve_destroy`)
//! - Choice presentation helpers (`present_target_choice`, `present_yes_no`, etc.)
//! - Target collection helpers (`any_targets`, `creature_targets`, etc.)
//! - Trigger-time condition helpers (`werewolf_should_trigger`, `morbid_should_trigger`)
//! - Library search (`search_library`, `finish_library_search`)

use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

/// Resolve an aura spell: attach it to the target creature on the battlefield.
/// If the target is no longer on the battlefield, the aura goes to graveyard.
/// Returns true if the aura was successfully attached.
pub fn resolve_aura(state: &mut GameState, aura_id: ObjectId, targets: &[Target], registry: &CardRegistry) -> bool {
    if let Some(Target::Object(target_id)) = targets.first() {
        if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.move_object(aura_id, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(aura_id) {
                obj.attached_to = Some(*target_id);
                obj.summoning_sick = false;
            }
            return true;
        }
    }
    false
}

/// Resolve a curse aura: attach to a target player and move to battlefield.
pub fn resolve_curse(state: &mut GameState, curse_id: ObjectId, targets: &[Target], registry: &CardRegistry) -> bool {
    if let Some(Target::Player(player_id)) = targets.first() {
        state.move_object(curse_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(curse_id) {
            obj.attached_to_player = Some(*player_id);
            obj.summoning_sick = false;
        }
        return true;
    }
    false
}

/// Resolve a damage spell: deal `amount` damage to the first target
/// (creature or player), then move the spell to the appropriate zone.
pub fn resolve_damage(state: &mut GameState, spell_id: ObjectId, targets: &[Target], amount: u32, registry: &CardRegistry) {
    if let Some(target) = targets.first() {
        let source_name = state.get_object(spell_id)
            .map_or_else(|| "spell".into(), |o| o.name.clone());
        let effect = PendingEffect::DealDamage {
            amount,
            source_id: spell_id,
            source_name,
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}

/// Resolve a targeted destruction spell: destroy the first target creature
/// via the destruction pipeline (checks indestructible/regeneration).
///
/// The spell's own trip to the graveyard is the engine's — see
/// [`GameState::resolving_spell`] — so this takes no spell id.
pub fn resolve_destroy(
    state: &mut GameState,
    targets: &[Target],
    registry: &crate::cards::CardRegistry,
) {
    if let Some(Target::Object(target_id)) = targets.first() {
        if let Some(obj) = state.get_object(*target_id) {
            if obj.zone == Zone::Battlefield {
                crate::destruction::try_destroy(state, *target_id, registry);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Choice presentation helpers
//
// These set up AwaitingAction::ResolutionChoice so the game loop
// asks the player to make a decision. The CLI and LLM player both
// know how to render these choices.
// ═══════════════════════════════════════════════════════════════════

/// Present a "choose one target" choice to the player.
///
/// - If `targets` is empty, does nothing.
/// - If mandatory (`optional == false`) and exactly 1 target, auto-applies the effect.
/// - Otherwise, sets up a `ResolutionChoice` for the player to pick.
pub fn present_target_choice(
    state: &mut GameState,
    source_id: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
    effect: PendingEffect,
    description: &str,
    optional: bool,
    registry: &CardRegistry,
) {
    if targets.is_empty() {
        return;
    }
    if targets.len() == 1 && !optional {
        // Mandatory with exactly 1 target — auto-apply. Through the caller's
        // registry: building a fresh one here rebuilt all 249 card behaviours
        // mid-resolution, and quietly ignored a caller that had registered an
        // extra card in its own.
        crate::engine::apply_pending_effect(state, &targets[0], &effect, registry);
        return;
    }
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: controller,
        source: source_id,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: description.into(),
            options: targets,
            optional,
            effect,
        },
    });
}

/// Present a "choose one target" choice that is optional ("you may").
pub fn present_optional_target_choice(
    state: &mut GameState,
    source_id: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
    effect: PendingEffect,
    description: &str,
    registry: &CardRegistry,
) {
    present_target_choice(state, source_id, controller, targets, effect, description, true, registry);
}

// ═══════════════════════════════════════════════════════════════════
// Target collection helpers
//
// Build lists of valid targets for common patterns.
// ═══════════════════════════════════════════════════════════════════

/// All targetable creatures on the battlefield (respects hexproof/protection).
#[must_use]
pub fn creature_targets(state: &GameState, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry))
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect()
}

/// All targetable creatures on the battlefield except a specific one.
#[must_use]
pub fn creature_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.id != exclude)
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect()
}

/// Every creature on the battlefield except one — for effects that CHOOSE a
/// creature rather than target it.
///
/// CR 115.1: an effect targets only where the word "target" appears. Hexproof
/// (702.11) and protection (702.16) restrict targeting, so neither applies
/// here. Evil Twin's "a copy of any creature on the battlefield" is a
/// replacement-effect choice (CR 614.12b), and an opponent's hexproof creature
/// is a legal thing to copy — using the targeting helper wrongly hid it.
#[must_use]
pub fn creature_choices_except(state: &GameState, exclude: ObjectId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.id != exclude)
        .filter(|o| state.is_creature(o.id, registry))
        .map(|o| Target::Object(o.id))
        .collect()
}

/// All targetable creatures + planeswalkers + all players ("any target").
#[must_use]
pub fn any_targets(state: &GameState, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    let mut targets = creature_targets(state, source_id, controller, registry);
    // Add planeswalkers (which have power = None, so creature_targets misses them)
    for o in state.objects.values() {
        if o.zone == Zone::Battlefield && !state.is_creature(o.id, registry)
            && state.has_card_type(o.id, crate::types::CardType::Planeswalker, registry)
            && crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry)
        {
            targets.push(Target::Object(o.id));
        }
    }
    for player in &state.players {
        if !state.player_has_hexproof(player.id, registry) || player.id == controller {
            targets.push(Target::Player(player.id));
        }
    }
    targets
}

/// All targetable creatures + planeswalkers + all players, excluding a specific object.
#[must_use]
pub fn any_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    let mut targets = creature_targets_except(state, exclude, source_id, controller, registry);
    // Add planeswalkers (which have power = None, so creature_targets misses them)
    for o in state.objects.values() {
        if o.zone == Zone::Battlefield && !state.is_creature(o.id, registry) && o.id != exclude
            && state.has_card_type(o.id, crate::types::CardType::Planeswalker, registry)
            && crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry)
        {
            targets.push(Target::Object(o.id));
        }
    }
    for player in &state.players {
        if !state.player_has_hexproof(player.id, registry) || player.id == controller {
            targets.push(Target::Player(player.id));
        }
    }
    targets
}

/// All creatures controlled by a specific player.
#[must_use]
pub fn creatures_controlled_by(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == player)
        .map(|o| Target::Object(o.id))
        .collect()
}

/// The single opponent in a 2-player game (auto-target convenience).
#[must_use]
pub fn opponent_player(state: &GameState, controller: PlayerId) -> Target {
    Target::Player(state.opponent(controller))
}

/// Get the controller of a permanent, with a fallback.
#[must_use]
pub fn controller_of(state: &GameState, object_id: ObjectId) -> PlayerId {
    state.get_object(object_id).map_or(PlayerId(0), |o| o.controller)
}

// ═══════════════════════════════════════════════════════════════════
// Transform helpers
//
// Generic transform logic for double-faced cards.
// ═══════════════════════════════════════════════════════════════════

/// Transform a double-faced permanent.
///
/// This flips `is_transformed` and nothing else that matters: every
/// characteristics accessor resolves through `GameState::face_data`, which
/// reads that flag, so the permanent's subtypes, keywords, card types and
/// colors all follow automatically. `name` is refreshed only because it is a
/// display cache for logging with no registry lookup behind it — rules code
/// reads `GameState::name_of`.
///
/// It used to copy the new face's name, keywords and subtypes onto the object,
/// which made those fields a second source of truth that a card hand-rolling
/// its own transform could leave stale. There is nothing left to leave stale.
pub fn apply_transform(state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
    let (card_id, was_transformed) = match state.get_object(object_id) {
        // A token copy of a double-faced card has only the copied face — it is
        // not itself a double-faced card, so it cannot transform (CR 111.7,
        // and the Back from the Brink ruling says so explicitly). A token
        // stamped with a DFC's `card_id` would otherwise pick up that card's
        // upkeep trigger and flip.
        Some(o) if o.zone == Zone::Battlefield && !o.is_token => (o.card_id, o.is_transformed),
        _ => return,
    };
    let Some(behavior) = registry.get(card_id) else { return; };

    // Refresh the display cache when the card declares a back face. Some DFCs
    // (Garruk Relentless) model their back face by branching on
    // `is_transformed` in `loyalty_abilities` / `dynamic_pt` instead of
    // declaring `back_face_data`, so the flip itself must not depend on one.
    let new_name = if was_transformed {
        Some(behavior.card_data().name)
    } else {
        behavior.back_face_data().map(|back| back.name)
    };

    if let Some(obj) = state.get_object_mut(object_id) {
        obj.is_transformed = !was_transformed;
        if let Some(name) = new_name {
            obj.name = name;
        }
    }
}

/// Format a tap plan as a short human-readable string like `tap 2x Swamp, Sol Ring`.
/// Groups identical names with a count prefix; returns an empty string if the
/// tap plan is empty. Engine-side analogue of `format_tap_plan` in mtg-player,
/// for use in resolution-prompt descriptions (e.g. Screeching Bat's may-pay).
#[must_use]
pub fn format_tap_plan_names(state: &GameState, tap_plan: &[(ObjectId, usize)]) -> String {
    if tap_plan.is_empty() {
        return String::new();
    }
    let names: Vec<String> = tap_plan.iter()
        .map(|&(id, _)| state.obj_name(id))
        .collect();
    // Group consecutive identical names into "Nx Name" form, preserving
    // the tap plan's order.
    let mut groups: Vec<(String, usize)> = Vec::new();
    for name in names {
        if let Some(last) = groups.last_mut() {
            if last.0 == name {
                last.1 += 1;
                continue;
            }
        }
        groups.push((name, 1));
    }
    let parts: Vec<String> = groups.into_iter()
        .map(|(n, c)| if c == 1 { n } else { format!("{c}x {n}") })
        .collect();
    format!("tap {}", parts.join(", "))
}

/// CR 603.4 intervening-if gate for the werewolf day/night transform trigger.
///
/// Every werewolf DFC in the set reads "At the beginning of each upkeep, **if**
/// [no spells were cast last turn | a player cast two or more spells last
/// turn], transform this creature." That `if` is an intervening-if clause: the
/// ability only triggers when the condition already holds at the beginning of
/// upkeep. When it doesn't, nothing goes on the stack — and so no player gets
/// the priority window that a stack entry would open.
///
/// Delegating to the card's own `should_transform` is what keeps the
/// dispatch-time check and the resolution-time check from ever disagreeing.
/// Werewolves pass this straight through from `CardBehavior::should_trigger`.
///
/// Only `Upkeep` is gated; a werewolf face with an unconditional trigger of
/// another kind (Howlpack Alpha's end-step Wolf token) still fires normally.
/// The Innistrad werewolf transform condition, both directions.
///
/// Front face: "At the beginning of each upkeep, if no spells were cast last
/// turn, transform this creature."
/// Back face: "...if a player cast two or more spells last turn, transform
/// this creature."
///
/// Twelve cards carried a byte-identical private copy of this, and every copy
/// carried the same invention: `&& !state.is_first_turn`. There is no such
/// clause in the oracle text. "No spells were cast last turn" is satisfied
/// when there was no last turn — zero spells were cast in it.
#[must_use]
// ═══════════════════════════════════════════════════════════════════
// Replacement effects (CR 614)
//
// Two shapes cover nearly every replacement a permanent has about its own
// arrival, so they are written once here rather than at each card.
// ═══════════════════════════════════════════════════════════════════

/// "This permanent enters the battlefield tapped unless <condition>"
/// (CR 614.1d) — the five Innistrad check lands.
pub fn enters_tapped_unless(
    self_id: ObjectId,
    event: &crate::replacement::ReplaceableEvent,
    untapped_if: impl FnOnce() -> bool,
) -> Option<crate::replacement::Replacement> {
    use crate::replacement::{ReplaceableEvent, Replacement};
    let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
    if e.object != self_id || e.tapped || untapped_if() {
        return None;
    }
    let mut e = e.clone();
    e.tapped = true;
    Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
}

/// "This permanent enters the battlefield with N counters on it"
/// (CR 614.1c). `counters` is only consulted for this permanent's own arrival.
pub fn enters_with_counters(
    self_id: ObjectId,
    event: &crate::replacement::ReplaceableEvent,
    counters: impl FnOnce() -> Vec<(crate::types::CounterType, u32)>,
) -> Option<crate::replacement::Replacement> {
    use crate::replacement::{ReplaceableEvent, Replacement};
    let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
    if e.object != self_id {
        return None;
    }
    let extra = counters();
    if extra.is_empty() {
        return None;
    }
    let mut e = e.clone();
    e.counters.extend(extra);
    Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
}

pub fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
    if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
        state.num_spells_cast_last_turn.values().any(|&count| count >= 2)
    } else {
        state.num_spells_cast_last_turn.values().sum::<u32>() == 0
    }
}

pub fn werewolf_should_trigger(
    behavior: &dyn CardBehavior,
    state: &GameState,
    self_id: ObjectId,
    kind: &TriggerKind,
    registry: &CardRegistry,
) -> bool {
    if *kind == TriggerKind::Upkeep {
        // A token copy of a werewolf cannot transform, so its transform
        // ability has nothing to do and should not reach the stack at all.
        if state.get_object(self_id).is_some_and(|o| o.is_token) {
            return false;
        }
        return behavior.should_transform(state, self_id, registry);
    }
    true
}

/// CR 603.4 intervening-if gate for the morbid enters-the-battlefield trigger.
///
/// "Morbid — When this creature enters, **if** a creature died this turn, ..."
/// is an intervening-if clause, so with no creature dead this turn the ability
/// doesn't trigger and no stack entry appears. Resolution handlers keep their
/// own `creature_died_this_turn` guard, which is what made the *outcome*
/// correct before this gate existed — only the phantom stack entry was wrong.
pub fn morbid_should_trigger(state: &GameState, kind: &TriggerKind) -> bool {
    if *kind == TriggerKind::EntersBattlefield {
        return state.creature_died_this_turn;
    }
    true
}

/// CR 701.19: search a library, take at most one matching card, then shuffle.
///
/// Handles the whole shape once: no match (shuffle and move on), exactly one
/// (take it without asking — there is no choice to make), or several (ask, and
/// finish in `finish_library_search` when the answer comes back). Five cards
/// each had their own copy of this, with subtly different behaviour in the
/// zero- and one-candidate cases.
///
/// `candidates` is what the card considers a legal find; `destination` and
/// `tapped` are where it ends up. The shuffle always happens, even when
/// nothing is found — you searched.
pub fn search_library(
    state: &mut GameState,
    source_id: ObjectId,
    searcher: PlayerId,
    candidates: Vec<ObjectId>,
    destination: Zone,
    tapped: bool,
    optional: bool,
    description: &str,
    registry: &CardRegistry,
) {
    // "You MAY search" is a real decision even when only one card qualifies,
    // or when nothing does — the player is entitled to decline, and a player
    // who declines never searched, so they do not shuffle either. Asking
    // regardless also keeps the prompt from leaking whether the library holds
    // a match: an unconditional shuffle told the table there was nothing to
    // find. With no candidates the only answer available is "decline", which
    // is the right outcome, not a reason to skip the question.
    if optional {
        let options: Vec<Target> = candidates.into_iter().map(Target::Object).collect();
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: searcher,
            source: source_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: description.to_string(),
                options,
                optional: true,
                effect: PendingEffect::FinishLibrarySearch { searcher, destination, tapped },
            },
        });
        return;
    }

    // Mandatory search from here on. CR 701.19: the search happened whether or
    // not anything was found, so the shuffle happens too.
    match candidates.len() {
        0 => {
            state.log(crate::state::LogLevel::Event,
                format!("{}: no matching card found in library", state.obj_name(source_id)));
            shuffle_library(state, searcher);
        }
        1 => finish_library_search(state, searcher, candidates[0], destination, tapped, registry),
        _ => {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: searcher,
                source: source_id,
                choice: ResolutionChoiceKind::ChooseFromLibrary {
                    description: description.to_string(),
                    options: candidates,
                    searcher,
                    source_id,
                    destination,
                    tapped,
                },
            });
        }
    }
}

/// Move a found card out of the library to its destination, then shuffle.
/// Shared by the auto-pick path in `search_library` and the engine's handler
/// for the player's answer, so both behave identically.
pub fn finish_library_search(
    state: &mut GameState,
    searcher: PlayerId,
    found: ObjectId,
    destination: Zone,
    tapped: bool,
    registry: &CardRegistry,
) {
    let name = state.obj_name(found);
    state.get_player_mut(searcher).library_order.retain(|&id| id != found);
    state.move_object(found, destination, registry);
    if destination == Zone::Battlefield {
        if let Some(obj) = state.get_object_mut(found) {
            obj.summoning_sick = false;
            if tapped {
                obj.tapped = true;
            }
        }
    }
    state.log(crate::state::LogLevel::Event,
        format!("p{} searched their library and found {name}", searcher.0));
    shuffle_library(state, searcher);
}

/// CR 701.20: shuffle a player's library.
pub fn shuffle_library(state: &mut GameState, player: PlayerId) {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    state.get_player_mut(player).library_order.shuffle(&mut rng);
}

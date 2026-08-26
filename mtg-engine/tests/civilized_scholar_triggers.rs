//! Regression test for Bug E (`BUG_REPORT_8SEAT.md)`: Civilized Scholar's
//! end-step transform-back trigger was defined on the front face and
//! fired every end step even when the creature was in its front face.
//! The trigger resolved as a no-op (`on_end_step` bails if !`is_transformed`)
//! but it still polluted the stack and consumed an LLM prompt every turn.
//!
//! The fix: triggered abilities are now looked up only on the currently
//! visible face (face-aware trigger collection), and Civilized Scholar's
//! `EndStep` trigger moved to the back-face (Homicidal Brute) where it
//! belongs per oracle.

mod common;
use common::*;
use mtg_engine::cards::helpers;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::Step;

const P0: PlayerId = PlayerId(0);

#[test]
fn front_face_civilized_scholar_has_no_end_step_trigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    assert!(!state.get_object(scholar).unwrap().is_transformed,
        "setup sanity: scholar should be on front face");

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger { source: TriggerSource { .. }, event: TriggerEvent::EndStep })
    )).count();
    assert_eq!(end_step_triggers, 0,
        "Front-face Civilized Scholar has no EndStep trigger per oracle");
}

#[test]
fn back_face_homicidal_brute_has_end_step_trigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);

    // Transform to Homicidal Brute (back face) via the shared helper.
    helpers::apply_transform(&mut state, scholar, &registry);
    assert!(state.get_object(scholar).unwrap().is_transformed);
    assert_eq!(state.get_object(scholar).unwrap().name, "Homicidal Brute");

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger { source: TriggerSource { .. }, event: TriggerEvent::EndStep })
    )).count();
    assert_eq!(end_step_triggers, 1,
        "Back-face Homicidal Brute should fire its end-step transform-back trigger");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Homicidal Brute's end step trigger is "if it didn't attack this turn, tap it
/// and transform it back". CR 711.5: transforming does NOT make a new object,
/// so a permanent that attacked as Civilized Scholar and then transformed HAS
/// attacked this turn, and stays transformed.
///
/// (An audit ticket claimed the opposite — that the Brute is a new entity and
/// the attack marker is stale. It isn't; the marker is stamped with the turn it
/// happened on, which is what distinguishes it from one left over from an
/// earlier turn.)
#[test]
fn an_attack_before_transforming_still_counts_for_the_back_face() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    state.active_player = P0;

    // Place Civilized Scholar, already transformed to Homicidal Brute
    let turn = state.turn_number;
    let brute = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    if let Some(obj) = state.get_object_mut(brute) {
        obj.is_transformed = true;
        obj.name = "Homicidal Brute".into();
        // It attacked earlier THIS turn, as Civilized Scholar. The marker is
        // stamped with the turn it happened on — a bare "has attacked" marker
        // could never be told apart from one left over from a previous turn.
        obj.card_state.insert("attacked_on_turn".into(),
            mtg_engine::ids::ObjectId(u64::from(turn)));
    }

    // Fire end step trigger — Brute should tap and transform back because
    // IT (Homicidal Brute) didn't attack this turn. The attack marker is from
    // before the transform (when it was Scholar).
    let behavior = registry.get(state.get_object(brute).unwrap().card_id).unwrap();
    behavior.on_end_step(&mut state, brute, &[], &registry);

    let is_still_transformed = state.get_object(brute).unwrap().is_transformed;
    assert!(is_still_transformed,
        "Homicidal Brute should stay transformed — the permanent attacked this turn (as Scholar)");
}

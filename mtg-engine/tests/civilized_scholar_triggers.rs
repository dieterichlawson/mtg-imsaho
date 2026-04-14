//! Regression test for Bug E (BUG_REPORT_8SEAT.md): Civilized Scholar's
//! end-step transform-back trigger was defined on the front face and
//! fired every end step even when the creature was in its front face.
//! The trigger resolved as a no-op (on_end_step bails if !is_transformed)
//! but it still polluted the stack and consumed an LLM prompt every turn.
//!
//! The fix: triggered abilities are now looked up only on the currently
//! visible face (face-aware trigger collection), and Civilized Scholar's
//! EndStep trigger moved to the back-face (Homicidal Brute) where it
//! belongs per oracle.

mod common;
use common::*;

use mtg_engine::cards::helpers;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::Step;

const P0: PlayerId = PlayerId(0);

#[test]
fn front_face_civilized_scholar_has_no_end_step_trigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let scholar = named_creature(&mut state, &registry, "Civilized Scholar", P0);
    assert!(!state.get_object(scholar).unwrap().is_transformed,
        "setup sanity: scholar should be on front face");

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger::EndStepTrigger { .. })
    )).count();
    assert_eq!(end_step_triggers, 0,
        "Front-face Civilized Scholar has no EndStep trigger per oracle");
}

#[test]
fn back_face_homicidal_brute_has_end_step_trigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let scholar = named_creature(&mut state, &registry, "Civilized Scholar", P0);

    // Transform to Homicidal Brute (back face) via the shared helper.
    helpers::apply_transform(&mut state, scholar, &registry);
    assert!(state.get_object(scholar).unwrap().is_transformed);
    assert_eq!(state.get_object(scholar).unwrap().name, "Homicidal Brute");

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger::EndStepTrigger { .. })
    )).count();
    assert_eq!(end_step_triggers, 1,
        "Back-face Homicidal Brute should fire its end-step transform-back trigger");
}

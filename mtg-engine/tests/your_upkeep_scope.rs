mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::CardId;
use mtg_engine::state::{GameState, StackEntry};
use mtg_engine::types::*;

// CR 603.2: "At the beginning of your upkeep/end step" constrains the
// trigger to the controller's step. The engine currently creates triggers
// for ALL permanents regardless of whose step it is.
//
// We use collect_triggers (not process_triggers/collect_step_triggers) so
// triggers land on the stack without being resolved. process_triggers
// resolves them in a loop, clearing the stack before we can observe them.

fn collect_step_triggers(state: &mut GameState, step: Step, registry: &CardRegistry) {
    state.events.push(GameEvent::StepStarted { step });
    mtg_engine::triggers::collect_triggers(state, registry);
}

fn has_trigger_for_card(state: &GameState, card_id: CardId) -> bool {
    let on_stack = state.stack.iter().any(|entry| {
        if let StackEntry::Trigger(t) = entry {
            t.behavior_card_id() == card_id
        } else {
            false
        }
    });
    let in_pending = state.pending_trigger_pushes_ap.iter()
        .chain(state.pending_trigger_pushes_nap.iter())
        .any(|t| t.behavior_card_id() == card_id);
    on_stack || in_pending
}

/// Every card whose upkeep or end-step trigger is scoped to its controller,
/// checked in both directions at once: it fires on the controller's step and
/// stays silent on the opponent's.
///
/// Six of these were written one card at a time, and every one asserted only
/// that nothing fired on the opponent's step — which a card with no trigger at
/// all would also satisfy. Sweeping the set covers the cards nobody typed out,
/// and asserting the positive direction is what makes the negative one mean
/// something.
#[test]
fn a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses() {
    let reg = registry();

    // (card name, the step its "your" trigger is scoped to)
    let mut cases: Vec<(String, Step)> = Vec::new();
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(behavior) = reg.get(id) else { continue };
        for ability in &behavior.card_data().triggered_abilities {
            let step = match ability.kind {
                mtg_engine::cards::TriggerKind::Upkeep => Step::Upkeep,
                mtg_engine::cards::TriggerKind::EndStep => Step::EndStep,
                _ => continue,
            };
            if behavior.step_trigger_scope(&ability.kind, false)
                == mtg_engine::cards::TriggerScope::Your
            {
                cases.push(((*name).to_string(), step));
            }
        }
    }
    cases.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| format!("{:?}", a.1).cmp(&format!("{:?}", b.1))));
    cases.dedup();
    assert!(cases.len() >= 6,
        "only {} cards declare a controller-scoped step trigger — this sweep has \
         stopped covering the set", cases.len());

    for (name, step) in &cases {
        let card = reg.get_id_by_name(name).unwrap();

        // P0 controls the permanent and is the active player: it fires.
        let mut own = game_at_step(*step, P0);
        let _id = named_permanent(&mut own, &reg, name, P0);
        // Several of these want a graveyard card to target, so give every one
        // something to find — a missing target must never be the reason
        // nothing fired.
        let _spirit = named_card_in_graveyard(&mut own, &reg, "Voiceless Spirit", P0);
        collect_step_triggers(&mut own, *step, &reg);
        assert!(has_trigger_for_card(&own, card),
            "{name}'s {step:?} trigger did not fire on its own controller's step");

        // Same board, opponent's step: it does not.
        let mut theirs = game_at_step(*step, P1);
        let _id = named_permanent(&mut theirs, &reg, name, P0);
        let _spirit = named_card_in_graveyard(&mut theirs, &reg, "Voiceless Spirit", P0);
        collect_step_triggers(&mut theirs, *step, &reg);
        assert!(!has_trigger_for_card(&theirs, card),
            "{name}'s \"your {step:?}\" trigger fired on the opponent's step (CR 603.2)");
    }
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// The end-to-end version of the same rule: the sweep above stops at trigger
/// collection, this one runs the full dispatch and checks the stack is clean.
#[test]
fn nothing_reaches_the_stack_during_an_opponents_upkeep() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P1); // P1's upkeep
    state.active_player = P1;

    // Place Charmbreaker Devils for P0 (NOT the active player)
    let _devils = named_permanent(&mut state, &registry, "Charmbreaker Devils", P0);

    // Process triggers during P1's upkeep
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    assert!(state.stack.is_empty(),
        "No trigger should be on the stack during opponent's upkeep, but stack has {} entries",
        state.stack.len());
}

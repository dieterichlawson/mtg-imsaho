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

/// Which step a trigger fires on is written on the card, so it should not also
/// be a judgement call in `step_trigger_scope`: "each upkeep" is
/// `TriggerScope::Each`, "your upkeep" is `Your`, "enchanted player's upkeep"
/// is `AttachedPlayer`. Read the expectation out of the oracle text and check
/// the declaration against it.
///
/// This is what makes the two behavioural sweeps below mean something. Each of
/// those only looks at the cards that already claim a given scope, so a card
/// scoped the wrong way simply moves from one sweep to the other and both stay
/// green. The oracle text is the outside opinion — and it is itself checked
/// against Scryfall by
/// `card_data_invariants.rs::oracle_text_says_what_scryfall_says`, so this
/// closes the loop without a hand-maintained list of card names.
#[test]
fn a_step_triggers_scope_is_the_one_its_oracle_text_states() {
    use mtg_engine::cards::{TriggerKind, TriggerScope};
    let reg = registry();

    let mut checked = 0usize;
    let mut offenders = Vec::new();
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(behavior) = reg.get(id) else { continue };
        for is_back_face in [false, true] {
            let data = if is_back_face {
                match behavior.back_face_data() { Some(d) => d, None => continue }
            } else {
                behavior.card_data()
            };
            let text = data.oracle_text.to_lowercase();
            for ability in &data.triggered_abilities {
                let word = match ability.kind {
                    TriggerKind::Upkeep => "upkeep",
                    TriggerKind::EndStep => "end step",
                    _ => continue,
                };
                let expected = if text.contains(&format!("beginning of each {word}")) {
                    TriggerScope::Each
                } else if text.contains(&format!("beginning of your {word}")) {
                    TriggerScope::Your
                } else if text.contains(&format!("beginning of enchanted player's {word}")) {
                    TriggerScope::AttachedPlayer
                } else {
                    // Some step triggers are worded another way entirely
                    // ("at the beginning of the end step of each player's
                    // turn"); this invariant has nothing to say about those.
                    continue;
                };
                checked += 1;
                let declared = behavior.step_trigger_scope(&ability.kind, is_back_face);
                if declared != expected {
                    let face = if is_back_face { " (back face)" } else { "" };
                    offenders.push(format!(
                        "{name}{face}: text says {expected:?} for the {word} trigger, \
                         `step_trigger_scope` says {declared:?}"));
                }
            }
        }
    }
    assert!(checked >= 40,
        "only {checked} step trigger(s) matched a known wording — this invariant \
         has stopped covering the set");
    assert!(offenders.is_empty(), "{} step trigger(s) are scoped against their own text:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// The other half of the same rule, and the half nothing asserted: a card that
/// says "at the beginning of EACH upkeep/end step" has to fire on both
/// players' steps. Reaper from the Abyss's morbid trigger and every werewolf's
/// day/night trigger are worded that way.
///
/// The sweep above proves the `Your` cards are scoped; without this one,
/// a `TriggerScope::Your` accidentally left on an "each" card — or the default
/// quietly changed to `Your` — would pass the whole file.
#[test]
fn an_each_step_trigger_fires_on_both_players_steps() {
    let reg = registry();

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
                == mtg_engine::cards::TriggerScope::Each
            {
                cases.push(((*name).to_string(), step));
            }
        }
    }
    cases.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| format!("{:?}", a.1).cmp(&format!("{:?}", b.1))));
    cases.dedup();
    assert!(cases.len() >= 6,
        "only {} cards declare an each-player step trigger — this sweep has \
         stopped covering the set", cases.len());

    for (name, step) in &cases {
        let card = reg.get_id_by_name(name).unwrap();

        for (active, whose) in [(P0, "its controller's"), (P1, "the opponent's")] {
            let mut state = game_at_step(*step, active);
            let _id = named_permanent(&mut state, &reg, name, P0);
            // Morbid clauses (CR 603.4) are checked at dispatch, so satisfy
            // them: a missing intervening-if must not be the reason nothing
            // fired.
            state.creature_died_this_turn = true;
            // Something to target, for the ones that do.
            let _victim = ready_creature(&mut state, P1, 3, 3);
            let _spirit = named_card_in_graveyard(&mut state, &reg, "Voiceless Spirit", P0);
            collect_step_triggers(&mut state, *step, &reg);
            assert!(has_trigger_for_card(&state, card),
                "{name}'s \"each {step:?}\" trigger did not fire on {whose} step");
        }
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

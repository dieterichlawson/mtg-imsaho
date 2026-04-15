mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::CardId;
use mtg_engine::state::{GameState, StackEntry};
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

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

// Oracle: "At the beginning of your upkeep, return target Spirit card
// from your graveyard to your hand."
// "Your upkeep" means only the controller's upkeep (CR 603.2).
#[test]
fn test_angel_of_flight_alabaster_trigger_only_on_your_upkeep() {
    let reg = registry();
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    // P1 is active (this is P1's upkeep). P0 controls Angel.
    let mut state = game_at_step(Step::Upkeep, P1);
    let _angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);

    // Put a Spirit in P0's graveyard so the trigger would have a valid target.
    let _spirit = named_card_in_graveyard(&mut state, &reg, "Voiceless Spirit", P0);

    collect_step_triggers(&mut state, Step::Upkeep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, angel_card), false,
        "Angel of Flight Alabaster should not trigger during opponent's upkeep (CR 603.2)"
    );
}

// Oracle: "At the beginning of your upkeep, target player draws a card
// and loses 1 life."
// "Your upkeep" means only the controller's upkeep.
#[test]
fn test_bloodgift_demon_trigger_only_on_your_upkeep() {
    let reg = registry();
    let demon_card = reg.get_id_by_name("Bloodgift Demon").unwrap();

    // P1 is active (this is P1's upkeep). P0 controls Bloodgift Demon.
    let mut state = game_at_step(Step::Upkeep, P1);
    let _demon = named_creature(&mut state, &reg, "Bloodgift Demon", P0);

    collect_step_triggers(&mut state, Step::Upkeep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, demon_card), false,
        "Bloodgift Demon should not trigger during opponent's upkeep (CR 603.2)"
    );
}

// Oracle (back face): "At the beginning of your end step, create a 2/2
// green Wolf creature token."
// "Your end step" means only the controller's end step.
#[test]
fn test_howlpack_alpha_end_step_trigger_only_on_your_end_step() {
    let reg = registry();
    let mayor_card = reg.get_id_by_name("Mayor of Avabruck").unwrap();

    // P1 is active (this is P1's end step). P0 controls Howlpack Alpha.
    let mut state = game_at_step(Step::EndStep, P1);
    let mayor = named_creature(&mut state, &reg, "Mayor of Avabruck", P0);
    state.get_object_mut(mayor).unwrap().is_transformed = true;

    collect_step_triggers(&mut state, Step::EndStep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, mayor_card), false,
        "Howlpack Alpha should not create Wolf tokens during opponent's end step (CR 603.2)"
    );
}

// Oracle: "At the beginning of your upkeep, mill two cards."
// "Your upkeep" means only the controller's upkeep.
#[test]
fn test_splinterfright_trigger_only_on_your_upkeep() {
    let reg = registry();
    let splinter_card = reg.get_id_by_name("Splinterfright").unwrap();

    // P1 is active (this is P1's upkeep). P0 controls Splinterfright.
    let mut state = game_at_step(Step::Upkeep, P1);
    let _splinter = named_creature(&mut state, &reg, "Splinterfright", P0);

    collect_step_triggers(&mut state, Step::Upkeep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, splinter_card), false,
        "Splinterfright should not trigger during opponent's upkeep (CR 603.2)"
    );
}

// Oracle (front face): "At the beginning of your upkeep, you may
// transform Cloistered Youth."
// "Your upkeep" means only the controller's upkeep.
#[test]
fn test_cloistered_youth_no_phantom_upkeep_trigger() {
    let reg = registry();
    let youth_card = reg.get_id_by_name("Cloistered Youth").unwrap();

    // P1 is active (this is P1's upkeep). P0 controls Cloistered Youth.
    let mut state = game_at_step(Step::Upkeep, P1);
    let _youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);

    collect_step_triggers(&mut state, Step::Upkeep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, youth_card), false,
        "Cloistered Youth should not trigger during opponent's upkeep (CR 603.2)"
    );
}

// Oracle (back face): "At the beginning of your end step, you lose 1 life."
// "Your end step" means only the controller's end step.
#[test]
fn test_unholy_fiend_no_phantom_end_step_trigger() {
    let reg = registry();
    let youth_card = reg.get_id_by_name("Cloistered Youth").unwrap();

    // P1 is active (this is P1's end step). P0 controls Unholy Fiend.
    let mut state = game_at_step(Step::EndStep, P1);
    let youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);
    state.get_object_mut(youth).unwrap().is_transformed = true;

    let life_before = state.players[0].life;

    collect_step_triggers(&mut state, Step::EndStep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, youth_card), false,
        "Unholy Fiend should not trigger during opponent's end step (CR 603.2)"
    );
    assert_eq!(
        state.players[0].life, life_before,
        "P0 should not lose life from Unholy Fiend during opponent's end step"
    );
}

// Oracle: "At the beginning of your upkeep, create X 2/2 black Zombie
// creature tokens, where X is half the number of Zombies you control,
// rounded down."
// "Your upkeep" means only the controller's upkeep.
#[test]
fn endless_ranks_no_phantom_trigger_opponent_upkeep() {
    let reg = registry();
    let ranks_card = reg.get_id_by_name("Endless Ranks of the Dead").unwrap();

    // P1 is active (this is P1's upkeep). P0 controls Endless Ranks.
    let mut state = game_at_step(Step::Upkeep, P1);
    let _ranks = named_creature(&mut state, &reg, "Endless Ranks of the Dead", P0);

    for _ in 0..4 {
        let z = ready_creature(&mut state, P0, 2, 2);
        let obj = state.get_object_mut(z).unwrap();
        obj.name = "Zombie".into();
        obj.is_token = true;
    }

    collect_step_triggers(&mut state, Step::Upkeep, &reg);

    assert_eq!(
        has_trigger_for_card(&state, ranks_card), false,
        "Endless Ranks of the Dead should not trigger during opponent's upkeep (CR 603.2)"
    );
}

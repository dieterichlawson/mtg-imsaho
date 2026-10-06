//! "Tap two untapped creatures you control" is one cost choice, asked as one
//! set when the ability is activated (CR 602.2b, 601.2h; issue #670).
//!
//! Skirsdag High Priest used to be one activated ability per pair of
//! creatures it could tap, the pair encoded in the ability index: 55 menu
//! rows at eleven creatures, 190 at twenty, on every surface, and the random
//! seat drew each pair as its own decision.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use mtg_engine::types::*;

fn priest_with(n: usize) -> (GameState, CardRegistry, ObjectId, Vec<ObjectId>) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    let priest = named_permanent(&mut state, &reg, "Skirsdag High Priest", P0);
    let others: Vec<ObjectId> = (0..n).map(|_| named_permanent(&mut state, &reg, "Grizzly Bears", P0)).collect();
    state.creature_died_this_turn = true;
    (state, reg, priest, others)
}

fn priest_offers(state: &GameState, reg: &CardRegistry, priest: ObjectId) -> Vec<Action> {
    mtg_engine::engine::legal_actions(state, reg).actions.into_iter()
        .filter(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == priest))
        .collect()
}

/// The checker has nothing to say about the activation. (Trigger scanning
/// is the game loop's, and these states come straight from `submit_action`.)
fn assert_quiet(state: &GameState, reg: &CardRegistry) {
    let complaints: Vec<String> = mtg_engine::invariants::check_core(state, reg).into_iter()
        .filter(|c| !c.contains("scanned for triggers"))
        .collect();
    assert!(complaints.is_empty(), "{complaints:?}");
}

fn cost_prompt(state: &GameState) -> Option<(Vec<ObjectId>, usize, usize)> {
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseObjectSet { options, min, max, effect: PendingEffect::PayActivationTaps { .. }, .. },
            ..
        }) => Some((options.clone(), *min, *max)),
        _ => None,
    }
}

/// The issue's board: eleven creatures besides the Priest is one row, not 55.
#[test]
fn the_priest_is_one_offer_however_many_creatures_could_pay() {
    let (state, reg, priest, _) = priest_with(11);
    assert_eq!(priest_offers(&state, &reg, priest).len(), 1);

    let (state, reg, priest, _) = priest_with(1);
    assert!(priest_offers(&state, &reg, priest).is_empty(), "one other creature cannot pay for two");
}

/// Activating asks for exactly two of the other untapped creatures, and pays
/// nothing until it has them.
#[test]
fn activating_asks_for_the_two_and_pays_nothing_yet() {
    let (mut state, reg, priest, others) = priest_with(4);
    state.get_object_mut(others[3]).unwrap().tapped = true;
    let asked = activate_onto_stack(&state, &reg, priest, None);
    let (options, min, max) = cost_prompt(&asked).expect("the cost is asked as one set");
    assert_eq!((min, max), (2, 2));
    assert_eq!(options, others[..3].to_vec(), "the other untapped creatures, and not the Priest");
    assert!(!asked.get_object(priest).unwrap().tapped, "nothing is paid while the cost is being chosen");
    assert!(asked.stack.is_empty());
    assert_quiet(&asked, &reg);
}

/// The answer pays the whole cost and puts the ability on the stack.
#[test]
fn the_chosen_two_are_tapped_with_the_priest() {
    let (state, reg, priest, others) = priest_with(4);
    let after = activate_tapping(&state, &reg, priest, &[others[0], others[2]]);
    assert!(after.awaiting_action.is_none());
    assert!(after.get_object(priest).unwrap().tapped, "{{T}}");
    assert!(after.get_object(others[0]).unwrap().tapped && after.get_object(others[2]).unwrap().tapped);
    assert!(!after.get_object(others[1]).unwrap().tapped, "only the chosen two");
    assert_eq!(after.stack.len(), 1, "the ability is on the stack");
    assert!(after.game_log.iter().any(|e| e.message.contains("tapped Grizzly Bears")),
        "the log names what paid: {:?}", after.game_log.iter().map(|e| &e.message).collect::<Vec<_>>());
    assert_quiet(&after, &reg);
}

/// A set that is not two offered, distinct, untapped creatures is refused,
/// and the question stays open.
#[test]
fn a_wrong_set_is_refused_and_the_prompt_stands() {
    let (state, reg, priest, others) = priest_with(3);
    let asked = activate_onto_stack(&state, &reg, priest, None);
    for bad in [vec![others[0]], vec![others[0], others[0]], vec![others[0], priest], vec![others[0], others[1], others[2]]] {
        let after = mtg_engine::engine::submit_action(&asked, &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenObjectSet(bad.clone()) }, &reg);
        assert!(cost_prompt(&after).is_some(), "{bad:?} was taken");
        assert!(!after.get_object(priest).unwrap().tapped, "{bad:?} paid something");
    }
}

/// Backing out costs nothing: nothing was paid.
#[test]
fn the_activation_can_be_cancelled() {
    let (state, reg, priest, others) = priest_with(2);
    let asked = activate_onto_stack(&state, &reg, priest, None);
    let after = mtg_engine::engine::submit_action(&asked, &Action::ResolveChoice {
        choice: ResolvedChoice::CancelCast }, &reg);
    assert!(after.awaiting_action.is_none());
    assert!(after.pending_ability_effect.is_none());
    assert!(after.stack.is_empty());
    assert!(!after.get_object(priest).unwrap().tapped);
    assert!(others.iter().all(|&o| !after.get_object(o).unwrap().tapped));
    assert_eq!(priest_offers(&after, &reg, priest).len(), 1, "and it is offered again");
}

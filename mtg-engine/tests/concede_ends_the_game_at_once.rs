//! CR 104.3a: "A player who concedes leaves the game immediately." A concede
//! (or the harness forfeiting a stalled seat) ends the game at the prompt it
//! is given at, including a choice asked from inside a resolution — not at
//! the next state-based check, which a pending resolution choice is re-asked
//! before (issue #764: the conceding player was asked the same question
//! again, and the effect finished after "p0 conceded").

mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::engine::submit_action;
use mtg_engine::invariants::check_core;
use mtg_engine::state::{AwaitingAction, GameResult};
use mtg_engine::types::*;

/// Night Terrors resolving against p1, stopped at p0's "choose a nonland
/// card to exile".
fn night_terrors_asking() -> (GameState, mtg_engine::cards::CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    let spell = castable_spell(&mut state, &reg, "Night Terrors", P0);
    spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);
    assert!(matches!(state.awaiting_action, Some(AwaitingAction::ResolutionChoice { player: P0, .. })),
        "test precondition: Night Terrors asks p0 to choose, got {:?}", state.awaiting_action);
    (state, reg)
}

fn hand_size(state: &GameState, p: PlayerId) -> usize {
    state.objects_in_id_order().into_iter().filter(|o| o.owner == p && o.zone == Zone::Hand).count()
}

#[test]
fn a_concede_at_a_resolution_choice_ends_the_game_there() {
    let (state, reg) = night_terrors_asking();
    let before = hand_size(&state, P1);
    let after = submit_action(&state, &Action::Concede, &reg);

    assert_eq!(after.result, Some(GameResult::Winner(P1)),
        "the concede ends the game at once; awaiting {:?}", after.awaiting_action);
    assert!(after.players[0].lost);
    assert_eq!(hand_size(&after, P1), before, "nothing of Night Terrors happens after the game ended");
    assert!(!after.game_log.iter().any(|l| l.message.contains("exiled")),
        "no exile is logged after the concede");
    // The final position is a real one: `--resume` refuses it as finished,
    // not as corrupt (#675, #765).
    assert_eq!(check_core(&after, &reg), Vec::<String>::new());
    assert_eq!(mtg_engine::invariants::check_transition(&state, Some(&Action::Concede), &after, &reg),
        Vec::<String>::new(), "the concede, as a transition (CR 104.2a: the game ended after the loss)");
}

#[test]
fn a_harness_forfeit_at_a_resolution_choice_ends_the_game_there() {
    let (state, reg) = night_terrors_asking();
    let after = submit_action(&state, &Action::Forfeit, &reg);
    assert_eq!(after.result, Some(GameResult::Winner(P1)),
        "a forfeit sent to a resolution choice ends the game, not re-asks it (the stall watchdog spun 100 times)");
    assert_eq!(after.players[0].loss_reason, Some(mtg_engine::events::LossReason::Forfeited));
    assert_eq!(check_core(&after, &reg), Vec::<String>::new());
}

/// Through the loop every runner drives: the conceding player is not asked
/// again, and nobody else is asked anything.
#[test]
fn the_loop_asks_nothing_after_a_concede_at_a_resolution_choice() {
    let (mut state, reg) = night_terrors_asking();
    let mut asked = Vec::new();
    mtg_engine::engine::run_game_loop(&mut state, &reg, |gs, acting, _legal| {
        asked.push((acting, format!("{:?}", gs.awaiting_action)));
        Action::Concede
    });
    assert_eq!(asked.len(), 1, "asked after the concede: {asked:#?}");
    assert_eq!(state.result, Some(GameResult::Winner(P1)));
}

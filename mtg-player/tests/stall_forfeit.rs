//! A runner's stopping move has to be sent, and has to stop the game.
//!
//! Two halves of one defect (issue #559), one in each crate:
//!
//! 1. `engine::legal_actions` returns early with the answers to an
//!    outstanding prompt — a mulligan, a discard, a declaration, any
//!    resolution choice — and only reaches the line that pushes `Concede`
//!    on the normal-priority path. For a set prompt the list it returns is
//!    empty. So `mtg-draft-runner`, which had both its stall forfeit and
//!    its 50,000-action ceiling behind `legal.actions.iter().position(...
//!    Concede)`, stopped a stalled game only when the stall happened to
//!    land on priority, and at every prompt did nothing at all.
//!
//! 2. Sending it was not enough either: `Action::Concede` conceded
//!    `state.priority_player`, which at a prompt is `None` (the whole
//!    mulligan phase, a declaration) or the *other* player (a blocker
//!    prompt). A forfeit at a mulligan conceded nobody, and the loop asked
//!    the same question again, forever.
//!
//! The seat below is the shape `engine.rs` promises the watchdog catches:
//! an answer the prompt does not offer is refused, said out loud, and the
//! same decision is asked again (issue #514). Nothing about the state
//! changes, so a runner is the only thing that can end it.

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, setup_game, Decklist, GameConfig, LegalActions};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::{AwaitingAction, GameState};
use mtg_player::watchdog::{self, ProgressWatchdog, STALLED_DECISIONS};

fn game() -> (GameState, CardRegistry) {
    let registry = CardRegistry::with_all_cards();
    let deck = Decklist {
        entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)],
    };
    let config = GameConfig {
        player_names: vec!["stuck".into(), "opp".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: Some(PlayerId(0)),
        rng_seed: Some(7),
    };
    let state = setup_game(&config, &registry);
    (state, registry)
}

/// A seat that spins at the first prompt it is given: it answers with a
/// row that prompt does not offer, which the engine refuses without
/// changing anything, and is asked again.
fn spinning_answer(_legal: &LegalActions) -> Action {
    Action::PassPriority
}

/// What the defect looked like from the runner's side: at the decision the
/// watchdog fires on, the move the runner reached for is not on the menu —
/// and the engine takes it anyway.
#[test]
fn the_stopping_move_is_not_on_the_menu_at_a_prompt() {
    let (mut state, registry) = game();
    let mut watchdog = ProgressWatchdog::new();
    let mut stall: Option<(String, bool, bool)> = None;
    let mut decisions = 0usize;

    let mut choose = |game_state: &GameState, _acting: PlayerId, legal: &LegalActions| -> Action {
        decisions += 1;
        assert!(
            decisions <= 10 * STALLED_DECISIONS as usize,
            "the spinning seat never stalled the game in {decisions} decisions"
        );
        if watchdog.observe(game_state) && stall.is_none() {
            stall = Some((
                legal.context.clone().unwrap_or_default(),
                legal.actions.iter().any(|a| matches!(a, Action::Concede)),
                legal.permits(&watchdog::forfeit_move()),
            ));
        }
        // Once the menu has been read there is nothing else to learn, so
        // end the game with the move the runners send.
        if stall.is_some() {
            return watchdog::forfeit_move();
        }
        spinning_answer(legal)
    };

    engine::run_game_loop(&mut state, &registry, &mut choose);

    let (asked, offers_concede, permits_concede) =
        stall.expect("the spinning seat should stall the game");
    assert!(
        !offers_concede,
        "the stall landed on a menu that offers Concede ({asked}), so this fixture no longer \
         exercises #559 — it needs a seat that spins at a prompt, not at priority"
    );
    assert!(
        permits_concede,
        "the engine refused the stopping move at {asked}; CR 104.3a says a concede is legal \
         at any time, and a runner has nothing else to stop a stalled game with"
    );
}

/// The property both runners need: a loop that *sends* the stopping move
/// at a stall comes back, and comes back with a finished game.
///
/// Before #559 this ran forever twice over — the draft runner looked the
/// move up in a list that does not hold it, and the engine's concede
/// landed on a `priority_player` that is `None` at a mulligan.
#[test]
fn a_stall_at_a_prompt_is_stopped_by_the_move_the_runner_sends() {
    // Room for the watchdog's own stalled decisions and the game before
    // them, and small enough that a loop with no termination condition
    // fails an assertion instead of hanging the suite.
    const DECISION_BOUND: usize = 10 * STALLED_DECISIONS as usize;

    let (mut state, registry) = game();
    let mut watchdog = ProgressWatchdog::new();
    let mut decisions = 0usize;
    let mut forfeited_at: Option<usize> = None;

    let mut choose = |game_state: &GameState, _acting: PlayerId, legal: &LegalActions| -> Action {
        decisions += 1;
        // A panic, not a stopping move: every move that could stop this
        // loop is the thing under test, so the escape hatch has to be one
        // the engine cannot swallow. A regression here is a failed test,
        // not a suite that hangs — which is what the defect does to an
        // operator.
        assert!(
            decisions <= DECISION_BOUND,
            "the game did not stop: {DECISION_BOUND} decisions and still going. The stopping \
             move was first sent at decision {forfeited_at:?} and the loop went on asking — \
             which is #559, whether because the runner never sent it or because the engine \
             dropped it"
        );
        // `mtg-draft-runner`'s guard, as it now stands.
        if watchdog.observe(game_state) {
            forfeited_at.get_or_insert(decisions);
            return watchdog::forfeit_move();
        }
        spinning_answer(legal)
    };

    engine::run_game_loop(&mut state, &registry, &mut choose);

    let forfeited_at = forfeited_at.expect("the spinning seat should stall the game");
    assert!(
        state.is_game_over(),
        "the loop returned after {decisions} decisions (forfeit first sent at {forfeited_at}) \
         but the game has no result: a forfeited game has to be a finished game, or the \
         tournament reports a forfeit it never resolved"
    );
}

/// The engine half on its own: whoever is being asked is who concedes.
#[test]
fn a_concede_at_a_prompt_concedes_the_player_being_asked() {
    let (state, registry) = game();

    // The mulligan phase: a prompt is outstanding and nobody holds
    // priority, which is the case that made the forfeit a silent no-op.
    let asked = state
        .player_to_act()
        .expect("the game opens on a mulligan decision");
    assert!(
        matches!(state.awaiting_action, Some(AwaitingAction::MulliganDecision { .. })),
        "expected the opening mulligan prompt, got {:?}",
        state.awaiting_action
    );
    assert_eq!(
        state.priority_player, None,
        "no player holds priority during the mulligan phase; that is the whole point of this \
         test, so if it has changed the fixture needs rebuilding"
    );

    let after = engine::submit_action(&state, &Action::Concede, &registry);
    assert!(
        after.players[asked.0 as usize].lost,
        "a concede at p{}'s mulligan prompt conceded nobody: the seat the harness is \
         forfeiting goes on playing and the prompt is asked again (issue #559)",
        asked.0
    );
}

/// The other way priority is the wrong answer: at a blocker prompt it is
/// the attacker's, so a forfeit read off it conceded the seat that was not
/// stuck.
#[test]
fn the_player_to_act_at_a_blocker_prompt_is_the_defender() {
    let (mut state, _registry) = game();
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers {
        defending_player: PlayerId(1),
    });
    state.priority_player = Some(PlayerId(0));

    assert_eq!(
        state.player_to_act(),
        Some(PlayerId(1)),
        "the defending player declares blockers; reading priority there names the attacker"
    );
}

/// The other stopping move, and the other runner. `mtg-runner` spends its
/// 50,000-action ceiling on an `AbandonGame` — the harness stopping rather
/// than a seat quitting (issue #233) — and the main loop returns on it.
/// The mulligan phase is a second loop that did not: it handed the action
/// to `permits` (which admits it) and `submit_action` (which is a no-op for
/// it) and asked the same mulligan again, so a game that stalled before
/// turn 1 had no ceiling at all.
#[test]
fn the_action_ceiling_stops_a_game_stalled_in_the_mulligan_phase() {
    const DECISION_BOUND: usize = 500;
    /// Where this run's "50,000 actions" falls.
    const CEILING_AT: usize = 50;

    let (mut state, registry) = game();
    let mut decisions = 0usize;

    let mut choose = |_s: &GameState, _acting: PlayerId, legal: &LegalActions| -> Action {
        decisions += 1;
        // As above: the escape hatch cannot be a stopping move, because the
        // stopping move is what is being tested.
        assert!(
            decisions <= DECISION_BOUND,
            "the loop went on asking after the harness abandoned the game at decision \
             {CEILING_AT}: a run that stalls in the mulligan phase has no ceiling (#559)"
        );
        if decisions >= CEILING_AT {
            return Action::AbandonGame;
        }
        spinning_answer(legal)
    };

    engine::run_game_loop(&mut state, &registry, &mut choose);

    assert!(
        state.result.is_none(),
        "abandoning is the harness stopping, not a seat losing: the game should be left \
         exactly as it stood, with nothing decided (issue #233), but the result is {:?}",
        state.result
    );
}

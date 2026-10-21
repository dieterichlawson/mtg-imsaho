//! A save is resumed, not replayed.
//!
//! The runner writes its save before every decision, and `--resume` hands
//! that state straight back to the game loop. Whatever the loop does on
//! entry — driving the opening hands, announcing turn 1, its first untap —
//! it has to do exactly once per game, whichever door the state came in
//! through. The loop tells a fresh game from a resumed one by a five-clause
//! test at its top, and loosening any clause of it makes a resumed game
//! start its first turn a second time: another `TurnStarted`, another
//! `StepStarted`, another round of turn-based actions, none of which the
//! uninterrupted game would have produced. The event ledger the seats read
//! and the invariant checker's turn accounting (`TurnStarted` events against
//! the turn counter) both see it.
//!
//! The property, then: the game that continues from a save is the game that
//! would have continued without the interruption — at every decision, and at
//! the end.

mod common;

use common::*;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig, LegalActions};
use mtg_engine::events::GameEvent;
use mtg_engine::types::*;

/// Eight Forests each: seven in hand, one in the library. The player on the
/// play skips their first draw (CR 103.7a), so the second player is the
/// first to draw from an empty library — on turn 4 — and loses to CR 704.5b.
/// A whole game, decided by the turn structure alone, in three land drops.
fn fresh_game() -> (GameState, CardRegistry) {
    let registry = registry();
    let deck = Decklist { entries: vec![("Forest".into(), 8)] };
    let config = GameConfig {
        player_names: vec!["P0".into(), "P1".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: None,
        rng_seed: Some(mtg_engine::state::default_rng_seed()),
    };
    (engine::setup_game(&config, &registry), registry)
}

/// The seat: keep every opening hand, play a land whenever one may be
/// played, attack and block with nothing, otherwise pass. Deterministic in
/// the state it is shown, which is what lets a resumed game be compared
/// with an uninterrupted one.
fn policy(legal: &LegalActions) -> Action {
    match legal.combat_prompt {
        Some(CombatPrompt::ChooseAttackers { .. }) =>
            return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] },
        Some(CombatPrompt::ChooseBlockers { .. }) =>
            return Action::DeclareBlockers { assignments: vec![] },
        None => {}
    }
    legal.actions.iter()
        .find(|a| matches!(a, Action::MulliganKeep | Action::PlayLand { .. }))
        .cloned()
        .unwrap_or(Action::PassPriority)
}

/// How the seat is shown the game: only the decisions the loop cannot make
/// for it, or (as under `--check-invariants`) every submitted action.
#[derive(Clone, Copy, Debug)]
enum Shown { Decisions, EverySubmit }

/// Drive `state` to the end of the game, returning the state the seat was
/// shown at each decision — the save the runner would have written there.
fn play_out(state: &mut GameState, registry: &CardRegistry, shown: Shown, resume: bool) -> Vec<GameState> {
    // A runtime setting, not part of the game: a loaded save does not carry
    // it, so it is set on the way in, as the runner sets it.
    state.observe_every_submit = matches!(shown, Shown::EverySubmit);
    let mut shown = Vec::new();
    let seat = |s: &GameState, _p: PlayerId, legal: &LegalActions| {
        shown.push(s.clone());
        policy(legal)
    };
    if resume {
        engine::resume_game_loop(state, registry, seat);
    } else {
        engine::run_game_loop(state, registry, seat);
    }
    shown
}

fn as_json(state: &GameState) -> serde_json::Value {
    serde_json::to_value(state).expect("a game state serializes")
}

/// The top-level fields on which two saves disagree, with the two values,
/// so a failure names what the interruption changed rather than dumping two
/// whole games.
fn differences(a: &GameState, b: &GameState) -> Vec<String> {
    let (a, b) = (as_json(a), as_json(b));
    let (Some(a), Some(b)) = (a.as_object(), b.as_object()) else {
        return vec!["a save is not a JSON object".into()];
    };
    a.iter()
        .filter(|(k, v)| b.get(*k) != Some(v))
        .map(|(k, v)| format!("{k}: {v} != {}", b.get(k).map_or("<absent>".to_string(), ToString::to_string)))
        .collect()
}

fn where_it_was(state: &GameState) -> String {
    format!("turn {} {:?}, {}", state.turn_number, state.step,
        match &state.awaiting_action {
            Some(a) => format!("awaiting {a:?}"),
            None => format!("priority {:?}", state.priority_player),
        })
}

/// Resumed at any of its decisions — the opening-hand ones included — the
/// game sees the same decisions from there on, in the same states, and ends
/// the same way. Checked both as a seat is shown the game and as the checker
/// is, where every pass is a decision and so a save can fall between any two
/// of them — an upkeep with priority just given, a step just entered.
#[test]
fn a_game_resumed_at_any_decision_continues_as_the_uninterrupted_game() {
    for shown in [Shown::Decisions, Shown::EverySubmit] {
        let (mut game, reg) = fresh_game();
        let decisions = play_out(&mut game, &reg, shown, false);
        assert!(game.is_game_over(), "{shown:?}: the fixture plays itself out");
        let turns_seen: Vec<u32> = decisions.iter().map(|s| s.turn_number).collect();
        assert!(decisions.iter().any(|s| engine::in_mulligan_phase(s)),
            "{shown:?}: the opening hands are decisions of their own: {turns_seen:?}");
        assert!(decisions.iter().filter(|s| !engine::in_mulligan_phase(s)).count() >= 3,
            "{shown:?}: a land drop on each of three turns: {turns_seen:?}");

        for (k, save) in decisions.iter().enumerate() {
            let at = where_it_was(save);
            // Through the save file, as the runner does it.
            let json = serde_json::to_string(save).expect("the save serializes");
            let mut resumed: GameState = serde_json::from_str(&json).expect("the save loads");
            let later = play_out(&mut resumed, &reg, shown, true);

            assert_eq!(later.len(), decisions.len() - k,
                "{shown:?}, resumed at decision {k} ({at}): the decisions still to come");
            for (i, (from_resume, from_run)) in later.iter().zip(&decisions[k..]).enumerate() {
                let diffs = differences(from_resume, from_run);
                assert!(diffs.is_empty(),
                    "{shown:?}, resumed at decision {k} ({at}): decision {} ({}) is not the \
                     one the uninterrupted game reached:\n  {}",
                    k + i, where_it_was(from_run), diffs.join("\n  "));
            }
            let diffs = differences(&resumed, &game);
            assert!(diffs.is_empty(),
                "{shown:?}, resumed at decision {k} ({at}): the game ends differently:\n  {}",
                diffs.join("\n  "));
        }
    }
}

/// Turn 1 is announced exactly once, when the opening hands are settled:
/// the first decision of the game proper carries `TurnStarted` for turn 1,
/// its untap step follows it, and no step starts before it (the checker's
/// "an untap step started without a turn starting"). Nothing later in the
/// game announces turn 1 again.
///
/// Seen as the checker sees it. A seat that is only shown its own decisions
/// has the loop pass for it through the upkeep, and each pass clears the
/// event buffer, so the announcement is gone by the time it is asked.
#[test]
fn the_first_turn_is_announced_once_before_its_first_step() {
    let (mut game, reg) = fresh_game();
    let decisions = play_out(&mut game, &reg, Shown::EverySubmit, false);

    let first = decisions.iter().find(|s| !engine::in_mulligan_phase(s))
        .expect("a decision after the opening hands");
    assert_eq!(first.turn_number, 1);
    let events = &first.events;
    let turn_started = events.iter().position(|e| matches!(e,
        GameEvent::TurnStarted { turn: 1, player } if *player == first.active_player));
    let Some(turn_started) = turn_started else {
        panic!("turn 1 was never announced to the seat: {events:?}");
    };
    assert!(!events[..turn_started].iter().any(|e| matches!(e, GameEvent::StepStarted { .. })),
        "no step starts before the turn does: {events:?}");
    assert!(matches!(events.get(turn_started + 1), Some(GameEvent::StepStarted { step: Step::Untap })),
        "turn 1 opens with its untap step, announced like every later step: {events:?}");

    let announcements = decisions.iter()
        .flat_map(|s| s.events.iter())
        .filter(|e| matches!(e, GameEvent::TurnStarted { turn: 1, .. }))
        .count();
    assert_eq!(announcements, 1, "turn 1 starts once");
}

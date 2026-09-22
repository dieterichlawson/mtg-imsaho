//! A set of cards is an answer the engine checks, not one it executes.
//!
//! `LegalActions::permits` admitted any `DiscardCards` at a discard prompt
//! and any `BottomCards` at a bottoming on the strength of the action's name
//! alone, and `simple::discard_cards` cleared the prompt whatever it was
//! handed — `state.awaiting_action = None` unconditionally, in a four-line
//! function. A seat that answered the empty set discarded nothing and the
//! engine believed the cleanup step had happened: the hand grew by one every
//! turn, the next cleanup asked for one more card than the last, and the game
//! played to a natural end with both players holding thirty-odd cards. No
//! panic, no invariant violation, no log line saying an answer was refused
//! (issue #567).
//!
//! CR 514.2 is the rule — "the active player discards down to that many
//! cards", not "may" — and `SetPrompt::accepts` was written to be exactly
//! this check ("the right number, all from the list, none of them twice").
//! It had no caller outside its own tests.
//!
//! No shipped seat reaches this: the CLI and the page build a set of the
//! required size, the LLM seat validates the model's indices and falls back
//! to the first `min` cards, and the fuzzer answers `min`, which for these
//! two prompts is the real count. It is reachable from anything speaking the
//! socket protocol, which is the class #514 was about, so the check belongs
//! in the engine rather than in a seat.

mod common;

use common::*;
use mtg_engine::actions::{Action, SetPromptKind};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig, LegalActions};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::{AwaitingAction, GameState};

/// The mirror from the issue's repro: nothing but lands and bears, so the
/// hand fills up and every cleanup asks for a discard.
fn mirror() -> (GameState, CardRegistry) {
    let registry = CardRegistry::with_all_cards();
    let deck = Decklist {
        entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)],
    };
    let config = GameConfig {
        player_names: vec!["empty".into(), "opp".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: Some(PlayerId(0)),
        rng_seed: Some(7),
    };
    let state = setup(&config, &registry);
    (state, registry)
}

fn setup(config: &GameConfig, registry: &CardRegistry) -> GameState {
    engine::setup_game(config, registry)
}

/// The seat from the issue: every set prompt is answered with the empty set,
/// everything else with the first row on the menu.
#[test]
fn a_seat_that_discards_nothing_does_not_get_to_keep_its_hand() {
    let (mut state, registry) = mirror();
    let mut decisions = 0usize;
    // Every `discard_count` the engine asked for, in order. Before the fix
    // this ran 1, 1, 2, 2, 3, 3, ... — one more card every turn, which is
    // the hand growing and nothing being discarded.
    let mut asked: Vec<usize> = Vec::new();

    let mut choose = |gs: &GameState, _p: PlayerId, legal: &LegalActions| -> Action {
        decisions += 1;
        // The refused answer leaves the decision unchanged, so the seat is
        // asked again and the game cannot end on its own. That is the #514
        // shape a runner's watchdog stops; here the cap is the stop.
        if decisions > 4_000 {
            return Action::AbandonGame;
        }
        if let Some(AwaitingAction::DiscardToHandSize { discard_count, .. }) = &gs.awaiting_action {
            asked.push(*discard_count);
        }
        if let Some(prompt) = legal.set_prompt.as_ref() {
            return prompt.answer(vec![]);
        }
        legal.actions.first().cloned().unwrap_or(Action::AbandonGame)
    };
    engine::run_game_loop(&mut state, &registry, &mut choose);

    let cleanup_asks: Vec<usize> = asked.clone();
    assert!(!cleanup_asks.is_empty(), "the game never reached a cleanup discard");
    let worst = cleanup_asks.iter().copied().max().unwrap_or(0);
    assert_eq!(worst, 1,
        "the cleanup asked for up to {worst} cards: the hand kept growing because \
         the empty answer was executed and the prompt cleared (CR 514.2)");
    assert!(cleanup_asks.len() > 1,
        "the discard prompt was asked once and never again — a refused answer has \
         to leave the decision standing, not consume it");
    for player in [PlayerId(0), PlayerId(1)] {
        let hand = state.objects_in_zone(mtg_engine::types::Zone::Hand, player).len();
        assert!(hand <= 8,
            "p{} ended holding {hand} cards", player.0);
    }
}

/// The contents check itself, at both prompts: the right number, all from
/// the list, none of them twice.
#[test]
fn a_set_prompt_refuses_an_answer_its_own_accepts_rejects() {
    let reg = registry();

    let discard = {
        let mut s = game_at_step(mtg_engine::types::Step::Cleanup, P0);
        let a = spell_in_hand(&mut s, &reg, "Grizzly Bears", P0);
        let b = spell_in_hand(&mut s, &reg, "Forest", P0);
        s.awaiting_action = Some(AwaitingAction::DiscardToHandSize {
            player: P0, discard_count: 1 });
        (mtg_engine::engine::legal_actions(&s, &reg), a, b)
    };
    let (legal, a, b) = discard;
    let prompt = legal.set_prompt.as_ref().expect("a discard prompt");
    assert_eq!(prompt.kind, SetPromptKind::DiscardToHandSize);
    assert!(legal.permits(&Action::DiscardCards { cards: vec![a] }),
        "one card out of the hand is the answer the prompt asked for");
    assert!(!legal.permits(&Action::DiscardCards { cards: vec![] }),
        "the empty set is one card short (CR 514.2 is not a 'may')");
    assert!(!legal.permits(&Action::DiscardCards { cards: vec![a, b] }),
        "two cards is one too many");
    assert!(!legal.permits(&Action::DiscardCards { cards: vec![a, a] }),
        "the same card twice is not two cards");
    assert!(!legal.permits(&Action::DiscardCards { cards: vec![mtg_engine::ids::ObjectId(9999)] }),
        "a card that is not in the hand is not in the list");

    let bottom = {
        let mut s = game_at_step(mtg_engine::types::Step::Upkeep, P0);
        let a = spell_in_hand(&mut s, &reg, "Grizzly Bears", P0);
        let b = spell_in_hand(&mut s, &reg, "Forest", P0);
        s.get_player_mut(P0).mulligan_count = 1;
        s.get_player_mut(P0).mulligan_kept = true;
        s.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 1 });
        (mtg_engine::engine::legal_actions(&s, &reg), a, b)
    };
    let (legal, a, b) = bottom;
    let prompt = legal.set_prompt.as_ref().expect("a bottoming prompt");
    assert_eq!(prompt.kind, SetPromptKind::BottomAfterMulligan);
    assert!(legal.permits(&Action::BottomCards { cards: vec![a] }));
    assert!(!legal.permits(&Action::BottomCards { cards: vec![] }),
        "bottoming nothing after a mulligan is not CR 103.4");
    assert!(!legal.permits(&Action::BottomCards { cards: vec![a, b] }));
    assert!(!legal.permits(&Action::BottomCards { cards: vec![a, a] }));
    assert!(!legal.permits(&Action::BottomCards { cards: vec![mtg_engine::ids::ObjectId(9999)] }));
}

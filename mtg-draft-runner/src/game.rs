//! The tournament's games: one match of `best_of` games between two seats,
//! each an LLM seat or a browser seat, with the watchdog, the forfeit and
//! the abandon rules the unattended runner has always had.
//!
//! The lockstep runner plays LLM seats only; the lobby
//! (`mtg-draft-server`) seats a person at a `GuiPlayer`. Both drive this
//! one loop, so a rule added for one is a rule the other has (the shape
//! #233 and #488 were about).

use mtg_draft::tournament::{self, GameOutcome, MatchResult};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;
use mtg_player::gui::GuiPlayer;
use mtg_player::llm::{LlmPlayer, MatchFormat};
use mtg_player::Player;

use crate::draft_log;
use crate::llm_client;

/// A seat at a tournament game.
pub enum GameSeat {
    Llm(LlmPlayer),
    Gui(GuiPlayer),
}

impl GameSeat {
    fn name(&self) -> &str {
        match self {
            GameSeat::Llm(p) => p.name(),
            GameSeat::Gui(p) => p.name(),
        }
    }

    /// The seat's own context for a fresh game. A page has none: it is
    /// sent the board.
    fn init_conversation(
        &mut self, deck: &[(String, u32)], registry: &CardRegistry, format: MatchFormat,
    ) {
        if let GameSeat::Llm(p) = self {
            // No set-wide card reference here. The game's system prompt
            // is re-read, cached, on every decision, and the whole set's
            // rules text is the largest thing it could carry — while every
            // card that comes into view is described in the decision
            // prompt itself (`Opp's cards in view`). The draft phase, where
            // the whole set is what is being chosen from, keeps the
            // reference.
            p.init_conversation(deck, "", registry, format);
        }
    }

    fn choose(
        &mut self, view: &GameView, legal: &engine::LegalActions,
    ) -> mtg_engine::actions::Action {
        match (self, &legal.combat_prompt) {
            (GameSeat::Llm(p), Some(prompt)) => p.choose_combat(view, prompt),
            (GameSeat::Llm(p), None) => p.choose_action(view, legal),
            (GameSeat::Gui(p), Some(prompt)) => p.choose_combat(view, legal, prompt),
            (GameSeat::Gui(p), None) => p.choose_action(view, legal),
        }
    }

    fn gave_up(&self) -> Option<String> {
        match self {
            GameSeat::Llm(p) => Player::gave_up(p),
            GameSeat::Gui(p) => Player::gave_up(p),
        }
    }

    /// Show a page the board while the other seat decides.
    fn observe(&mut self, view: &GameView) {
        if let GameSeat::Gui(p) = self {
            p.observe(view);
        }
    }

    fn game_over(&mut self, view: &GameView, summary: &str) {
        if let GameSeat::Gui(p) = self {
            p.game_over(view, summary);
        }
    }
}

/// One side of a match.
pub struct MatchSeat<'a> {
    pub seat: usize,
    pub deck: &'a Decklist,
    pub player: GameSeat,
}

/// The seed a match is played under: a function of the run's seed and the
/// match's own coordinates, so the same round pairs the same seats under
/// the same shuffles (issue #212).
#[must_use]
pub fn match_seed(root: u64, round: usize, seat_a: usize, seat_b: usize) -> u64 {
    let mut z = root
        ^ (round as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (seat_a as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (seat_b as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Whether a match of `best_of` games is decided, given what has been played.
///
/// Two ways a match ends, and the loop used to know only the first (#484):
/// somebody has won more than half the games, or `best_of` games have been
/// played. A drawn game wins nothing but is still a game played — MTR 6.5
/// ends a best-of-three after three games however they went — so without the
/// second clause a match with draws in it has no bound on its length, and an
/// even `--best-of` plays one game more than it says.
#[must_use]
pub fn match_is_over(best_of: usize, games_played: usize, wins_a: usize, wins_b: usize) -> bool {
    let needed = tournament::wins_needed(best_of);
    wins_a >= needed || wins_b >= needed || games_played >= best_of
}

/// Play one match. `after_game` is told each game's outcome as it ends, so
/// a surface that shows a match in progress can.
pub fn play_match(
    mut a: MatchSeat<'_>,
    mut b: MatchSeat<'_>,
    registry: &CardRegistry,
    best_of: usize,
    seed: u64,
    after_game: &mut dyn FnMut(&GameOutcome),
) -> MatchResult {
    let mut wins_a = 0;
    let mut wins_b = 0;
    let mut games: Vec<GameOutcome> = Vec::new();

    let seat_a = a.seat;
    let seat_b = b.seat;

    // Play/draw per MTG tournament rules, delegated to the engine helpers:
    //   Game 1: a fair coin flip.
    //   Games 2+: engine::next_starter_loser_plays() — the loser of the
    //   previous game always elects to play first (the strategically
    //   dominant choice in Limited); drawn games keep the previous starter.
    //
    // The flip and each game's engine seed come off this match's own RNG
    // rather than the thread's, so a seeded run replays its games and not
    // only its packs (issue #212).
    let mut match_rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(seed);
    let mut starter = PlayerId(u8::from(rand::Rng::gen_bool(&mut match_rng, 0.5)));

    while !match_is_over(best_of, games.len(), wins_a, wins_b) {
        let game_number = games.len() + 1;
        let outcome = play_game(
            &mut a,
            &mut b,
            registry,
            starter,
            MatchFormat::BestOf { best_of, game: game_number, your_wins: wins_a, their_wins: wins_b },
            MatchFormat::BestOf { best_of, game: game_number, your_wins: wins_b, their_wins: wins_a },
            rand::Rng::gen(&mut match_rng),
        );

        // Engine's winner is a PlayerId (0 = seat_a, 1 = seat_b).
        let prev_winner: Option<PlayerId> = outcome.winner.map(|w| {
            if w == seat_a { PlayerId(0) } else { PlayerId(1) }
        });
        starter = engine::next_starter_loser_plays(starter, prev_winner, 2);

        match outcome.winner {
            Some(w) if w == seat_a => wins_a += 1,
            Some(_) => wins_b += 1,
            None => {}
        }

        after_game(&outcome);
        games.push(outcome);

        // A seat whose backend gave up forfeits the rest of the match, not
        // only the game it gave up in: the games it would have played are
        // its opponent's, recorded as forfeits so the standings and
        // `=== Forfeited Games ===` say so (#587). Nothing is asked of it.
        let dead = [(seat_a, &a.player), (seat_b, &b.player)].into_iter()
            .find(|(_, p)| p.gave_up().is_some())
            .map(|(seat, _)| seat);
        if let Some(dead) = dead {
            let winner = if dead == seat_a { seat_b } else { seat_a };
            while !match_is_over(best_of, games.len(), wins_a, wins_b) {
                if winner == seat_a { wins_a += 1 } else { wins_b += 1 }
                let forfeit = GameOutcome {
                    winner: Some(winner), turns: 0, game_log: Vec::new(),
                    stalled_seat: Some(dead), abandoned: false,
                };
                after_game(&forfeit);
                games.push(forfeit);
            }
        }
    }

    MatchResult {
        player_a: seat_a,
        player_b: seat_b,
        wins_a,
        wins_b,
        games,
    }
}

#[allow(clippy::too_many_arguments)]
fn play_game(
    a: &mut MatchSeat<'_>,
    b: &mut MatchSeat<'_>,
    registry: &CardRegistry,
    starting_player: PlayerId,
    // Each seat's own side of the match: the score is stated from the seat's
    // point of view, so the two are mirrors of each other (issue #609).
    format_a: MatchFormat,
    format_b: MatchFormat,
    rng_seed: u64,
) -> GameOutcome {
    let seat_a = a.seat;
    let seat_b = b.seat;
    let (deck_a, deck_b) = (a.deck, b.deck);
    let (p1, p2) = (&mut a.player, &mut b.player);
    let config = GameConfig {
        player_names: vec![p1.name().to_string(), p2.name().to_string()],
        decklists: vec![deck_a.clone(), deck_b.clone()],
        starting_life: 20,
        starting_player: Some(starting_player),
        // Derived from the match's seed, so the shuffles replay (issue #212).
        rng_seed: Some(rng_seed),
    };

    let mut state = engine::setup_game(&config, registry);

    // Re-initialize conversations for this game (fresh context per game).
    // The context is fresh, so whatever the seat is to know about the match
    // around this game has to be in the system prompt — it is the only thing
    // that survives (issue #609).
    p1.init_conversation(&deck_a.entries, registry, format_a);
    p2.init_conversation(&deck_b.entries, registry, format_b);

    let mut action_count: u64 = 0;
    let max_actions: u64 = 50_000;

    // The progress watchdog `mtg-runner` has had since #462. This loop is
    // the other copy, and it had only the 50,000-action cap — which for a
    // pod of `cc` seats is 50,000 `claude -p` subprocesses spent re-asking
    // one question, and then a silent concede. A stalled game here forfeits
    // for the seat that is stuck and says so everywhere the game is
    // reported, rather than killing the tournament around it (#488).
    let mut watchdog = mtg_player::watchdog::ProgressWatchdog::new();
    let mut stalled_seat: Option<usize> = None;
    let mut abandoned = false;

    let mut game_callback =
        |game_state: &GameState,
         acting_player: PlayerId,
         legal: &engine::LegalActions|
         -> mtg_engine::actions::Action {
            action_count += 1;

            let stalled = watchdog.observe(game_state);
            if stalled && stalled_seat.is_none() {
                let seat = if acting_player == PlayerId(0) { seat_a } else { seat_b };
                stalled_seat = Some(seat);
                let report = mtg_player::watchdog::stall_report(
                    game_state, acting_player, legal, &seat.to_string(),
                );
                mtg_player::stderr_line!("\nWARN: {report} The game is forfeit to seat {}.",
                    if acting_player == PlayerId(0) { seat_b } else { seat_a });
                draft_log::DraftLogger::stalled_game(
                    seat_a, seat_b, seat,
                    game_state.turn_number,
                    &format!("{:?}", game_state.step),
                    &report,
                    file!(), line!(),
                );
            }

            if let Some(stop) = harness_move(stalled, action_count, max_actions) {
                if matches!(stop, mtg_engine::actions::Action::AbandonGame) && !abandoned {
                    abandoned = true;
                    mtg_player::stderr_line!("\nWARN: Seat {seat_a} vs Seat {seat_b}: the game reached \
{max_actions} actions without a result at turn {} {:?}; the runner abandoned it — no winner.",
                        game_state.turn_number, game_state.step);
                    draft_log::DraftLogger::abandoned_game(
                        seat_a, seat_b, max_actions,
                        game_state.turn_number,
                        &format!("{:?}", game_state.step),
                        file!(), line!(),
                    );
                }
                return stop;
            }

            // A page on the other side of the table is shown the board
            // while this seat decides, as `mtg-runner` shows it.
            let (player, other) = if acting_player == PlayerId(0) {
                (&mut *p1, &mut *p2)
            } else {
                (&mut *p2, &mut *p1)
            };
            if matches!(other, GameSeat::Gui(_)) {
                let other_id = PlayerId(1 - acting_player.0);
                other.observe(&GameView::for_player(game_state, other_id, registry));
            }

            let view = GameView::for_player(game_state, acting_player, registry);
            let answer = player.choose(&view, legal);
            // A seat whose backend spent its whole retry budget without an
            // answer has stopped playing: it forfeits this game through the
            // stall path, and `play_match` forfeits it the rest of the match.
            // Not a silent degrade onto fallbacks, and not a fatal that stops
            // every other match in the process (#587).
            if let Some(why) = player.gave_up() {
                if stalled_seat.is_none() {
                    let seat = if acting_player == PlayerId(0) { seat_a } else { seat_b };
                    stalled_seat = Some(seat);
                    let report = format!("Seat {seat}'s backend never answered within its retry \
budget ({why}), so the seat forfeits its match");
                    mtg_player::stderr_line!("\nWARN: {report}.");
                    draft_log::DraftLogger::stalled_game(
                        seat_a, seat_b, seat,
                        game_state.turn_number,
                        &format!("{:?}", game_state.step),
                        &report,
                        file!(), line!(),
                    );
                }
                return mtg_player::watchdog::forfeit_move();
            }
            answer
        };

    engine::run_game_loop(&mut state, registry, &mut game_callback);

    let winner = state.result.as_ref().and_then(|r| {
        match r {
            mtg_engine::state::GameResult::Winner(pid) => {
                if *pid == PlayerId(0) {
                    Some(seat_a)
                } else {
                    Some(seat_b)
                }
            }
            mtg_engine::state::GameResult::Draw => None,
        }
    });

    // A page is told how it ended, in `mtg-runner`'s words: the board's
    // p-number with the table's seat beside it, and how the loser lost —
    // conceded, forfeited, decked (#743).
    let seat_of = |id: PlayerId| if id == PlayerId(0) { seat_a } else { seat_b };
    let summary = match mtg_player::game_over_headline(&state, |id| format!("p{} (Seat {})", id.0, seat_of(id))) {
        Some(headline) => format!("{headline}\nFinal turn: {}", state.turn_number),
        None if abandoned => format!("Game abandoned: {max_actions} actions without a result — no winner."),
        None => format!("Game ended without a result.\nFinal turn: {}", state.turn_number),
    };
    // And the log is told, as `mtg-runner`'s is: how the game ended was
    // otherwise said only to a page, never recorded (#743).
    mtg_player::game_log::write(file!(), line!(), "RESULT", &summary);
    for (pid, player) in [(PlayerId(0), &mut *p1), (PlayerId(1), &mut *p2)] {
        if matches!(player, GameSeat::Gui(_)) {
            player.game_over(&GameView::for_player(&state, pid, registry), &summary);
        }
    }

    // Capture game log, filtering out Debug-level entries (priority passes etc.)
    // to keep the log readable
    let game_log: Vec<String> = state
        .game_log
        .iter()
        .filter(|entry| entry.level as u8 >= 1) // Info and above
        .map(|entry| entry.message.clone())
        .collect();

    GameOutcome {
        winner,
        turns: state.turn_number,
        game_log,
        stalled_seat,
        abandoned,
    }
}

/// The move the runner itself makes, when it makes one, before the acting
/// seat is asked anything.
///
/// The two reasons to stop are not the same event. A seat the watchdog
/// caught spinning forfeits: the loss is that seat's, and the report says
/// so. A game that is still moving but has run out of budget is the
/// harness stopping, and nobody wins it — sending the forfeit there handed
/// the game to whichever seat was *not* acting at action 50,000 (#630,
/// the draft runner's copy of #233).
///
/// Either is sent, not looked up. `legal.actions` lists `Concede` only on
/// the normal-priority path, so reaching for it there made both no-ops at
/// every prompt — a mulligan, a discard, a declaration, any resolution
/// choice — which is where a spinning seat usually is, leaving the game
/// with no termination condition at all (issue #559). The engine accepts
/// either at any decision point (CR 104.3a, `LegalActions::permits`).
#[must_use]
pub fn harness_move(stalled: bool, action_count: u64, max_actions: u64) -> Option<mtg_engine::actions::Action> {
    if stalled {
        Some(mtg_player::watchdog::forfeit_move())
    } else if action_count >= max_actions {
        Some(mtg_player::watchdog::ceiling_move())
    } else {
        None
    }
}

/// The LLM game seat a model spec names, as the game harness plays it.
#[must_use]
pub fn make_game_player(model_spec: &str, name: &str, guide: Option<&str>) -> LlmPlayer {
    // Parse "provider:model:draft_thinking:game_thinking"
    let parts: Vec<&str> = model_spec.split(':').collect();
    let provider = parts[0];
    let model = parts.get(1).copied();
    // Game thinking is the 4th part, or falls back to 3rd, or defaults
    let game_thinking = parts.get(3).or(parts.get(2)).copied();

    let mut p = match provider {
        "gemini" => {
            let mut p = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "claude" => {
            let mut p = LlmPlayer::new(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "claude-code" | "cc" => {
            let mut p = LlmPlayer::new_claude_code(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        // Unreachable once validate_model_specs has run, and fatal if it ever
        // is reached: substituting a seat plays a different game than the one
        // requested and still prints a winner.
        other => crate::die(&format!(
            "unknown model provider '{other}' (expected {})",
            llm_client::ACCEPTED_PROVIDERS
        )),
    };
    if let Some(level) = game_thinking {
        p = p.with_thinking_level(level);
    }
    if let Some(g) = guide {
        p = p.with_guide(g.to_string());
    }
    p
}

#[cfg(test)]
mod match_length_tests {
    use super::match_is_over;

    /// #484: `--best-of 2` played three games, because only a win target
    /// ended the match and two wins are needed to take a two-game match.
    #[test]
    fn an_even_best_of_stops_at_the_games_it_names() {
        assert!(!match_is_over(2, 0, 0, 0));
        assert!(!match_is_over(2, 1, 1, 0));
        // 1-1 after both games: the match is over and drawn, not extended.
        assert!(match_is_over(2, 2, 1, 1));
        // Winning both still ends it at two.
        assert!(match_is_over(2, 2, 2, 0));
    }

    /// A drawn game wins nothing but is still a game played, so a match with
    /// draws in it is bounded (MTR 6.5) instead of running forever.
    #[test]
    fn drawn_games_still_count_toward_the_match_length() {
        // best-of-three, two draws and a win: 1-0 with three games played.
        assert!(!match_is_over(3, 1, 0, 0));
        assert!(!match_is_over(3, 2, 0, 0));
        assert!(match_is_over(3, 3, 1, 0));
        // And a best-of-three that draws every game ends drawn.
        assert!(match_is_over(3, 3, 0, 0));
    }

    #[test]
    fn a_decided_match_does_not_play_its_dead_game() {
        assert!(match_is_over(3, 2, 2, 0));
        assert!(!match_is_over(3, 2, 1, 1));
        assert!(match_is_over(1, 1, 1, 0));
    }
}

#[cfg(test)]
mod harness_stop_tests {
    use super::harness_move;
    use crate::standings::unplayed_games_note;
    use mtg_draft::tournament::GameOutcome;
    use mtg_engine::actions::Action;

    /// #630: the action ceiling and the stall forfeit shared one move, so a
    /// game still progressing at action 50,000 was conceded on behalf of
    /// whichever seat held the decision — the loss #233 removed from
    /// `mtg-runner`. The ceiling is the harness stopping; nobody wins it.
    #[test]
    fn the_action_ceiling_abandons_the_game_rather_than_conceding_it() {
        assert!(harness_move(false, 49_999, 50_000).is_none());
        assert!(matches!(harness_move(false, 50_000, 50_000), Some(Action::AbandonGame)),
            "the ceiling must not be a seat's concede");
        // A stalled seat still forfeits: that loss is the stuck seat's, and
        // recorded as a forfeit, not as a concede it never chose (#742).
        assert!(matches!(harness_move(true, 10, 50_000), Some(Action::Forfeit)));
        assert!(matches!(harness_move(true, 50_000, 50_000), Some(Action::Forfeit)));
    }

    #[test]
    fn an_abandoned_game_is_named_on_the_score_line() {
        let game = |stalled_seat, abandoned| GameOutcome {
            winner: None, turns: 30, game_log: vec![], stalled_seat, abandoned,
        };
        assert_eq!(unplayed_games_note(&[game(None, false)]), "");
        assert_eq!(unplayed_games_note(&[game(None, true)]),
            " [1 game abandoned: the action budget ran out, no winner]");
        assert_eq!(unplayed_games_note(&[game(Some(1), false), game(None, true), game(None, true)]),
            " [1 game forfeited: a seat stalled] [2 games abandoned: the action budget ran out, no winner]");
        // A forfeit the table made before any game began says why, as its
        // log does, not "a seat stalled" (#759).
        let unplayed = |why: &str| GameOutcome {
            winner: Some(1), turns: 0, game_log: vec![format!("forfeit: {why}")], stalled_seat: Some(0), abandoned: false,
        };
        assert_eq!(unplayed_games_note(&[unplayed("no game page for seat 0"), unplayed("no game page for seat 0")]),
            " [2 games forfeited: no game page for seat 0]");
        assert_eq!(unplayed_games_note(&[game(Some(0), false), unplayed("the seat is away")]),
            " [1 game forfeited: a seat stalled] [1 game forfeited: the seat is away]");
    }
}

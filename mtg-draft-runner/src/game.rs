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
use mtg_engine::types::Zone;
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
        match self {
            // No set-wide card reference here. The game's system prompt
            // is re-read, cached, on every decision, and the whole set's
            // rules text is the largest thing it could carry — while every
            // card that comes into view is described in the decision
            // prompt itself (`Opp's cards in view`). The draft phase, where
            // the whole set is what is being chosen from, keeps the
            // reference.
            GameSeat::Llm(p) => p.init_conversation(deck, "", registry, format),
            // A page is told which game this is and how the match stands,
            // which the LLM seat's system prompt says and a person returning
            // to the page was never told (#855).
            GameSeat::Gui(p) => p.page_handle().set_match_line(match_line(format)),
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

    /// Why a page's seat was taken out of its game (a hosted table's kick,
    /// #821); an LLM seat that gave up is not withdrawn.
    fn withdrawn(&self) -> Option<String> {
        match self {
            GameSeat::Llm(_) => None,
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

/// "Game 2 of a best-of-3 · you 1, them 0": the match around a game, from
/// one seat's side, as a page shows it.
#[must_use]
pub fn match_line(format: MatchFormat) -> Option<String> {
    match format {
        MatchFormat::SingleGame => None,
        MatchFormat::BestOf { best_of, game, your_wins, their_wins } => Some(if game == 1 {
            format!("Game 1 of a best-of-{best_of}")
        } else {
            format!("Game {game} of a best-of-{best_of} · you {your_wins}, them {their_wins}")
        }),
    }
}

/// One side of a match.
pub struct MatchSeat {
    pub seat: usize,
    /// How the host's terminal names the seat: `Seat N` for the runner,
    /// the hosted table's "Ann (seat 3)" (#816). The log keeps `Seat N`.
    pub label: String,
    /// The deck it plays, as registered; sideboarding between games
    /// replaces it for the rest of the match.
    pub deck: Decklist,
    pub player: GameSeat,
}

/// What a match knows before each game: who chooses to play or draw, and
/// how the match stands.
pub struct BeforeGame<'a> {
    /// 1-based number of the game about to be played.
    pub game: usize,
    /// The seat that chooses whether to play or draw: the coin flip's
    /// winner for game 1, the loser of the game before after that, and the
    /// same seat again after a drawn game (MTR 2.2).
    pub chooser: usize,
    pub games: &'a [GameOutcome],
    /// Game wins so far: seat `a`'s, seat `b`'s.
    pub wins: (usize, usize),
    /// The cards each seat has shown its opponent in the match so far —
    /// on the battlefield, the stack, in a graveyard or exile by a game's
    /// end — seat `a`'s first. A sideboard is built against these.
    pub shown: &'a [Vec<String>; 2],
    /// The decks being played now: seat `a`'s, seat `b`'s.
    pub decks: [&'a Decklist; 2],
}

/// What the seats decided before a game.
#[derive(Default)]
pub struct GameSetup {
    /// A seat's deck from this game on, when it sideboarded.
    pub deck_a: Option<Decklist>,
    pub deck_b: Option<Decklist>,
    /// Whether the chooser plays first; `None` is the usual choice in
    /// Limited, to play.
    pub chooser_plays: Option<bool>,
}

/// "you lost game 2 on turn 11": how a game went, from one seat's side.
#[must_use]
pub fn last_game_line(game: &GameOutcome, seat: usize, number: usize) -> String {
    let how = match game.winner {
        Some(w) if w == seat => "you won",
        Some(_) => "you lost",
        None if game.abandoned => "no winner in",
        None => "you drew",
    };
    let forfeit = match game.stalled_seat {
        Some(s) if s == seat => " (you forfeited)",
        Some(_) => " (your opponent forfeited)",
        None => "",
    };
    format!("{how} game {number} on turn {}{forfeit}", game.turns)
}

/// Before every game: nobody sideboards and the chooser plays, which is
/// what a match did before sideboarding existed.
pub fn no_sideboarding(_: &BeforeGame) -> GameSetup {
    GameSetup::default()
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

/// Whether every saved match (its round and result) is one this tournament
/// finished: a score its
/// own games add up to, decided at the match length, and a pairing the
/// tournament makes in the round it names, given the matches before it.
///
/// A record was carried when its round and seats matched a pairing, and
/// taken as it stood: a 7-0 best-of-1 went into the standings, and a record
/// with its seats the other way round, for a seat or a round the pod does
/// not have, or twice over, was dropped in silence — the match it stood for
/// re-played and re-billed (#731). The pairings are a function of the
/// results, so the tournament is walked here on the records alone.
pub fn check_finished_matches(matches: &[(usize, &MatchResult)], players: usize, best_of: usize) -> Result<(), String> {
    let name = |&(round, r): &(usize, &MatchResult)| format!("round {round}, seat {} v seat {}", r.player_a, r.player_b);
    for (i, m) in matches.iter().enumerate() {
        let r = m.1;
        if matches[..i].iter().any(|o| o.0 == m.0
            && o.1.player_a == r.player_a && o.1.player_b == r.player_b)
        {
            return Err(format!("it has two records for {}", name(m)));
        }
        if r.games.iter().any(|g| g.winner.is_some_and(|w| w != r.player_a && w != r.player_b)) {
            return Err(format!("its record for {} has a game won by a seat not in the match", name(m)));
        }
        let won = |seat: usize, games: &[GameOutcome]| games.iter().filter(|g| g.winner == Some(seat)).count();
        let (a, b) = (won(r.player_a, &r.games), won(r.player_b, &r.games));
        if (r.wins_a, r.wins_b) != (a, b) {
            return Err(format!("its record for {} says {}-{}, and its games say {a}-{b}",
                name(m), r.wins_a, r.wins_b));
        }
        let before = &r.games[..r.games.len().saturating_sub(1)];
        let finished = !r.games.is_empty()
            && match_is_over(best_of, r.games.len(), a, b)
            && !match_is_over(best_of, before.len(), won(r.player_a, before), won(r.player_b, before));
        if !finished {
            return Err(format!("its record for {} is {a}-{b} over {} game(s), which is not a finished \
best-of-{best_of}", name(m), r.games.len()));
        }
    }
    let mut tournament = tournament::Tournament::new(players, tournament::TournamentConfig { best_of });
    let mut used = vec![false; matches.len()];
    while !tournament.is_complete() {
        let round = tournament.rounds.len() + 1;
        let pairings = tournament.generate_pairings();
        let real: Vec<(usize, usize)> = pairings.iter().filter(|&&(_, b)| b != tournament::BYE).copied().collect();
        let mut results = Vec::new();
        for &(a, b) in &real {
            let found = matches.iter()
                .position(|m| m.0 == round && m.1.player_a == a && m.1.player_b == b);
            if let Some(i) = found {
                used[i] = true;
                results.push(matches[i].1.clone());
            }
        }
        if results.len() < real.len() {
            break;
        }
        tournament.record_round(pairings, results);
    }
    match used.iter().position(|&u| !u) {
        Some(i) => Err(format!("its record for {} is not a match this tournament pairs", name(&matches[i]))),
        None => Ok(()),
    }
}

/// Play one match. `before_game` is asked before each game who plays
/// first and what each seat plays — the chooser's play or draw, and any
/// sideboarding between games (MTR 2.2, 2.4) — and `after_game` is told
/// each game's outcome as it ends, so a surface that shows a match in
/// progress can.
pub fn play_match(
    mut a: MatchSeat,
    mut b: MatchSeat,
    registry: &CardRegistry,
    best_of: usize,
    seed: u64,
    before_game: &mut dyn FnMut(&BeforeGame) -> GameSetup,
    after_game: &mut dyn FnMut(&GameOutcome),
) -> MatchResult {
    let mut wins_a = 0;
    let mut wins_b = 0;
    let mut games: Vec<GameOutcome> = Vec::new();

    let seat_a = a.seat;
    let seat_b = b.seat;

    // Play/draw per MTG tournament rules (MTR 2.2): the winner of a coin
    // flip chooses for game 1, the loser of each game for the next, and the
    // same player again after a drawn game. The choice is `before_game`'s;
    // without one the chooser plays, the usual choice in Limited.
    //
    // The flip and each game's engine seed come off this match's own RNG
    // rather than the thread's, so a seeded run replays its games and not
    // only its packs (issue #212).
    let mut match_rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(seed);
    let mut chooser = if rand::Rng::gen_bool(&mut match_rng, 0.5) { seat_b } else { seat_a };
    let mut shown: [Vec<String>; 2] = [Vec::new(), Vec::new()];

    while !match_is_over(best_of, games.len(), wins_a, wins_b) {
        let game_number = games.len() + 1;
        let setup = before_game(&BeforeGame {
            game: game_number, chooser, games: &games, wins: (wins_a, wins_b), shown: &shown,
            decks: [&a.deck, &b.deck],
        });
        if let Some(deck) = setup.deck_a {
            a.deck = deck;
        }
        if let Some(deck) = setup.deck_b {
            b.deck = deck;
        }
        let starts = if setup.chooser_plays.unwrap_or(true) { chooser } else if chooser == seat_a { seat_b } else { seat_a };
        let starter = if starts == seat_a { PlayerId(0) } else { PlayerId(1) };
        let (mut outcome, showed) = play_game(
            &mut a,
            &mut b,
            registry,
            starter,
            MatchFormat::BestOf { best_of, game: game_number, your_wins: wins_a, their_wins: wins_b },
            MatchFormat::BestOf { best_of, game: game_number, your_wins: wins_b, their_wins: wins_a },
            rand::Rng::gen(&mut match_rng),
        );

        // The loser chooses for the next game; after a draw, the same seat.
        if let Some(w) = outcome.winner {
            chooser = if w == seat_a { seat_b } else { seat_a };
        }
        for (all, these) in shown.iter_mut().zip(showed) {
            for card in these {
                if !all.contains(&card) {
                    all.push(card);
                }
            }
        }

        match outcome.winner {
            Some(w) if w == seat_a => wins_a += 1,
            Some(_) => wins_b += 1,
            None => {}
        }

        // A withdrawn seat's forfeit says why, the way the hosted table's
        // other forfeits do ("forfeit: <why>" first, which the standings
        // note and the table's game line read), not "a seat stalled".
        let withdrawn = [(seat_a, &a.player), (seat_b, &b.player)].into_iter()
            .find_map(|(seat, p)| p.withdrawn().map(|why| (seat, format!("forfeit: {why}"))));
        if let Some((seat, line)) = &withdrawn {
            if outcome.stalled_seat == Some(*seat) {
                outcome.game_log.insert(0, line.clone());
            }
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
                    winner: Some(winner), turns: 0,
                    game_log: withdrawn.iter().map(|(_, line)| line.clone()).collect(),
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
    a: &mut MatchSeat,
    b: &mut MatchSeat,
    registry: &CardRegistry,
    starting_player: PlayerId,
    // Each seat's own side of the match: the score is stated from the seat's
    // point of view, so the two are mirrors of each other (issue #609).
    format_a: MatchFormat,
    format_b: MatchFormat,
    rng_seed: u64,
) -> (GameOutcome, [Vec<String>; 2]) {
    let seat_a = a.seat;
    let seat_b = b.seat;
    let (label_a, label_b) = (a.label.clone(), b.label.clone());
    let label_of = |seat: usize| if seat == seat_a { label_a.clone() } else { label_b.clone() };
    let (deck_a, deck_b) = (&a.deck, &b.deck);
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
    let mut forfeit_after_loop: Option<PlayerId> = None;

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
                // The report names the seat `Seat N` for the log; the
                // terminal names it as the table does.
                mtg_player::stderr_line!("\nWARN: {} The game is forfeit to {}.",
                    report.replacen(&format!("Seat {seat}"), &label_of(seat), 1),
                    label_of(if acting_player == PlayerId(0) { seat_b } else { seat_a }));
                draft_log::DraftLogger::stalled_game(
                    false, seat_a, seat_b, seat,
                    game_state.turn_number,
                    &format!("{:?}", game_state.step),
                    &report,
                    file!(), line!(),
                );
            }

            if let Some(stop) = harness_move(stalled, action_count, max_actions) {
                if matches!(stop, mtg_engine::actions::Action::AbandonGame) && !abandoned {
                    abandoned = true;
                    mtg_player::stderr_line!("\nWARN: {label_a} vs {label_b}: the game reached \
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
            let other_id = PlayerId(1 - acting_player.0);
            let (player, other) = if acting_player == PlayerId(0) {
                (&mut *p1, &mut *p2)
            } else {
                (&mut *p2, &mut *p1)
            };
            let seat_of = |id: PlayerId| if id == PlayerId(0) { seat_a } else { seat_b };
            // The seat NOT being asked was taken out (a kick while its
            // opponent held the decision, #854): it is not asked again until
            // its opponent passes it something, which may be never, and the
            // opponent could concede the game to it meanwhile. The game is
            // stopped here and forfeit against it once the loop returns.
            if let Some(why) = other.gave_up() {
                if stalled_seat.is_none() {
                    stalled_seat = Some(seat_of(other_id));
                    report_gave_up(other, seat_of(other_id), &why, seat_a, seat_b, &label_of, game_state);
                }
                forfeit_after_loop = Some(other_id);
                return mtg_player::watchdog::ceiling_move();
            }
            if matches!(other, GameSeat::Gui(_)) {
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
                    stalled_seat = Some(seat_of(acting_player));
                    report_gave_up(player, seat_of(acting_player), &why, seat_a, seat_b, &label_of, game_state);
                }
                return mtg_player::watchdog::forfeit_move();
            }
            // Taken out while this seat was deciding: its answer (a concede
            // among them) does not count; the game is the withdrawn seat's
            // loss (#854).
            if let Some(why) = other.gave_up() {
                if stalled_seat.is_none() {
                    stalled_seat = Some(seat_of(other_id));
                    report_gave_up(other, seat_of(other_id), &why, seat_a, seat_b, &label_of, game_state);
                }
                forfeit_after_loop = Some(other_id);
                return mtg_player::watchdog::ceiling_move();
            }
            answer
        };

    engine::run_game_loop(&mut state, registry, &mut game_callback);
    // The seat withdrawn while the other held the decision loses the game
    // as a seat forfeiting at its own decision does.
    if let Some(pid) = forfeit_after_loop {
        if state.result.is_none() {
            state.player_loses(pid, mtg_engine::events::LossReason::Forfeited);
            state.end_game_if_decided();
        }
    }

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
    // The engine's word for a forfeit is "stopped answering"; a page taken
    // out of the game (a kick, #821) is told what actually happened, as
    // the one reason: the summary gave both, one after the other (#825).
    let mut summary = summary;
    for (pid, player) in [(PlayerId(0), &*p1), (PlayerId(1), &*p2)] {
        if let Some(why) = player.withdrawn() {
            let engine_words = format!("p{} (Seat {}) {}", pid.0, seat_of(pid),
                mtg_engine::events::FORFEIT_DESCRIPTION);
            let ours = format!("p{} (Seat {}) forfeited: withdrawn, {why}", pid.0, seat_of(pid));
            if summary.contains(&engine_words) {
                summary = summary.replacen(&engine_words, &ours, 1);
            } else {
                summary.push_str(&format!("\nSeat {} was withdrawn: {why}.", seat_of(pid)));
            }
        }
    }
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

    // What each seat showed its opponent: its cards (not tokens) that were
    // public when the game ended.
    let showed = [PlayerId(0), PlayerId(1)].map(|p| {
        let mut cards: Vec<String> = state.objects.values()
            .filter(|o| o.owner == p && !o.is_token)
            .filter(|o| matches!(o.zone, Zone::Battlefield | Zone::Graveyard | Zone::Exile | Zone::Stack))
            .map(|o| mtg_draft::front_face(&o.name).to_string())
            .collect();
        cards.sort();
        cards.dedup();
        cards
    });

    (GameOutcome {
        winner,
        turns: state.turn_number,
        game_log,
        stalled_seat,
        abandoned,
    }, showed)
}

/// Say on the terminal and in the log that a seat stopped playing: its
/// backend gave up, or its page was withdrawn (a kick, #821).
fn report_gave_up(
    player: &GameSeat, seat: usize, why: &str, seat_a: usize, seat_b: usize,
    label_of: &dyn Fn(usize) -> String, game_state: &GameState,
) {
    let report = match player {
        GameSeat::Llm(_) => format!("Seat {seat}'s backend never answered within its retry \
budget ({why}), so the seat forfeits its match"),
        // A page's seat taken out of the game: a kick (#821).
        GameSeat::Gui(_) => format!("Seat {seat} was withdrawn from the game ({why}), \
so the seat forfeits its match"),
    };
    // The terminal names the seat as the table does; the log's report
    // keeps `Seat N`.
    let said = report.replacen(&format!("Seat {seat}"), &label_of(seat), 1);
    mtg_player::stderr_line!("\nWARN: {said}.");
    draft_log::DraftLogger::stalled_game(
        matches!(player, GameSeat::Gui(_)),
        seat_a, seat_b, seat,
        game_state.turn_number,
        &format!("{:?}", game_state.step),
        &report,
        file!(), line!(),
    );
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

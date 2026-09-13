//! The progress watchdog: a bound on a game that has stopped moving.
//!
//! A seat that answers the same unusable thing forever does not fail — it
//! spins. The engine offers a cast, the seat names no target, the engine
//! rightly refuses 0 where 1 was required (CR 601.2c), cancels the cast and
//! returns priority, and the same cast is offered again. 2,180 cancelled
//! casts in 60 seconds, stuck on turn 15 (issue #462); in a tournament
//! where every seat is an LLM seat, 2,724 of them in 230 seconds with an
//! empty stderr (issue #488).
//!
//! This lives here rather than in either runner because there are two
//! runners: `mtg-runner` grew the watchdog when #462 was fixed and
//! `mtg-draft-runner`, which keeps its own copy of the game loop, did not —
//! the same shape as #404, a fix that did not travel between two copies of
//! one protocol. One implementation, both loops.

use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;

/// How many decisions that change nothing a game gets before it is declared
/// stuck.
///
/// Generous on purpose. Passing priority round the table leaves the state
/// alone for a decision or two, and nothing legitimate holds every life
/// total, zone count, mana pool, tap and damage mark still for a hundred
/// decisions running.
pub const STALLED_DECISIONS: u32 = 100;

/// Everything one decision could move, as one number.
///
/// Two decisions with the same fingerprint changed nothing any player can
/// see: same turn and step, same stack, same lives and libraries and hands
/// and graveyards, same permanents in the same states, same floating mana.
/// The priority holder is deliberately absent — the loop this catches hands
/// priority back to the same seat every time, and a ping-pong that changed
/// nothing else would be just as stuck.
#[must_use]
pub fn progress_fingerprint(state: &GameState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    state.turn_number.hash(&mut h);
    format!("{:?}", state.step).hash(&mut h);
    state.stack.len().hash(&mut h);
    for p in &state.players {
        p.life.hash(&mut h);
        p.land_plays_remaining.hash(&mut h);
        p.lost.hash(&mut h);
        p.library_order.len().hash(&mut h);
        p.graveyard_order.len().hash(&mut h);
        p.mana_pool.total().hash(&mut h);
    }
    // Sorted by id: `objects` is a map, and its iteration order is not the
    // game's (see #402 for what reading a map's order as an order costs).
    let mut objects: Vec<_> = state.objects.values().collect();
    objects.sort_by_key(|o| o.id);
    for o in &objects {
        o.id.hash(&mut h);
        format!("{:?}", o.zone).hash(&mut h);
        o.controller.hash(&mut h);
        o.tapped.hash(&mut h);
        o.summoning_sick.hash(&mut h);
        o.damage_marked.hash(&mut h);
    }
    h.finish()
}

/// Watches a game loop for decisions that change nothing.
///
/// Call [`ProgressWatchdog::observe`] once per decision, before asking the
/// seat. It returns `true` the moment the game has been standing still for
/// [`STALLED_DECISIONS`] decisions; what to do about that is the runner's
/// call — `mtg-runner` stops the run, the tournament forfeits the game for
/// the seat that is stuck — but the diagnosis is the same, and so is
/// [`stall_report`], which says what the seat is being asked.
#[derive(Debug, Default)]
pub struct ProgressWatchdog {
    last_fingerprint: Option<u64>,
    stalled: u32,
}

impl ProgressWatchdog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one decision. `true` once the game has stopped making
    /// progress, and every decision after that until something moves.
    pub fn observe(&mut self, state: &GameState) -> bool {
        let fingerprint = progress_fingerprint(state);
        if Some(fingerprint) == self.last_fingerprint {
            self.stalled += 1;
        } else {
            self.last_fingerprint = Some(fingerprint);
            self.stalled = 0;
        }
        self.stalled >= STALLED_DECISIONS
    }

    /// How many decisions in a row have changed nothing.
    #[must_use]
    pub fn stalled_decisions(&self) -> u32 {
        self.stalled
    }
}

/// What a stalled game is stuck on: the seat, the question, where in the
/// game it is, and the last thing that happened. Without it the operator
/// gets a run that never ends and an empty stderr.
#[must_use]
pub fn stall_report(
    state: &GameState,
    acting_player: PlayerId,
    legal: &mtg_engine::engine::LegalActions,
    seat: &str,
) -> String {
    let asked = legal.context.clone().unwrap_or_else(|| {
        if legal.combat_prompt.is_some() {
            "a combat prompt".into()
        } else {
            "priority".into()
        }
    });
    let last = state
        .game_log
        .last()
        .map(|e| e.message.clone())
        .unwrap_or_default();
    format!(
        "the game stopped making progress: {STALLED_DECISIONS} decisions in a row left it \
exactly as they found it. Seat {seat} (p{}) is being asked {asked} at turn {} {:?}, and \
answering it the same unusable way every time. Last game-log entry: {last}",
        acting_player.0, state.turn_number, state.step,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtg_engine::cards::CardRegistry;
    use mtg_engine::engine::{setup_game, Decklist, GameConfig};

    fn game() -> (GameState, CardRegistry) {
        let registry = CardRegistry::with_all_cards();
        let deck = Decklist {
            entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)],
        };
        let config = GameConfig {
            player_names: vec!["you".into(), "opp".into()],
            decklists: vec![deck.clone(), deck],
            starting_life: 20,
            starting_player: Some(PlayerId(0)),
            rng_seed: Some(7),
        };
        let state = setup_game(&config, &registry);
        (state, registry)
    }

    /// A game that is not moving trips the watchdog, and only after the
    /// documented number of decisions — 100 stalled decisions, not 100
    /// decisions.
    #[test]
    fn a_state_that_never_changes_trips_the_watchdog() {
        let (state, _registry) = game();
        let mut watchdog = ProgressWatchdog::new();

        // The first observation establishes the fingerprint; the stall count
        // is decisions *after* it that changed nothing.
        for decision in 0..STALLED_DECISIONS {
            assert!(
                !watchdog.observe(&state),
                "tripped after {decision} identical decisions, before {STALLED_DECISIONS}"
            );
        }
        assert!(watchdog.observe(&state));
        assert_eq!(watchdog.stalled_decisions(), STALLED_DECISIONS);
    }

    /// Anything a player can see moving resets it, so a long game of real
    /// decisions never trips.
    #[test]
    fn a_game_that_moves_resets_the_watchdog() {
        let (mut state, _registry) = game();
        let mut watchdog = ProgressWatchdog::new();

        for _ in 0..(STALLED_DECISIONS * 3) {
            for _ in 0..10 {
                assert!(!watchdog.observe(&state));
            }
            // One life point is enough: the fingerprint is everything a
            // decision could move.
            state.players[0].life -= 1;
            assert!(!watchdog.observe(&state));
        }
    }

    /// The report names the seat, the question and where the game is, which
    /// is the whole difference between a bounded run and a silent one.
    #[test]
    fn the_report_says_what_the_game_is_stuck_on() {
        let (state, registry) = game();
        let legal = mtg_engine::engine::legal_actions(&state, &registry);
        let report = stall_report(&state, PlayerId(0), &legal, "Seat0");

        assert!(report.contains("Seat Seat0 (p0)"), "{report}");
        assert!(report.contains("turn 1"), "{report}");
        assert!(report.contains("100 decisions in a row"), "{report}");
    }
}

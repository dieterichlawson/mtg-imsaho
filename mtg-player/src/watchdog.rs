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

/// The board, as one number: everything about the game a player can look
/// at and see.
///
/// Two decisions with the same fingerprint left the board alone. Hashed:
/// the turn and step; the stack's height; each player's life, land plays,
/// loss, library size, graveyard *in order*, and mana pool by colour; each
/// object's id, name, zone, controller, tapped and summoning-sick flags,
/// damage, counters, attachment, power and toughness, keywords, card types
/// and subtypes, regeneration shields, and which face is up; and all of
/// combat — who is attacking whom, who is blocking, and the order damage is
/// assigned in.
///
/// Two things are left out on purpose, and the distinction is the whole
/// design:
///
/// * **Whose decision it is, and the one in flight.** Priority, the pass
///   count, the trigger cursor, the prompt outstanding, a cast part-way
///   through announcement (its targets, its X, its chosen mode). These are
///   exactly what *does* move while a seat spins — the engine offers a
///   cast, the seat names no target, the cast is cancelled and offered
///   again (#462) — so hashing them would make this function blind to the
///   thing it exists to catch.
/// * **Library order.** No player can see it, and a fingerprint that
///   changes when a library is shuffled would score a repeating shuffle as
///   progress.
///
/// Everything else a player can see belongs here, and the list has grown
/// twice by being wrong: combat, because a damage-assignment order is a
/// decision that touches nothing else and the engine re-asks it once per
/// blocker, so two gang blocks read as a hundred identical decisions and
/// killed a live game at turn 29 (#509); and the object characteristics
/// above, because a werewolf flipping — a different name, different power
/// and toughness, different abilities — was scored as "nothing moved"
/// (#560). The maps are ordered, so the hash is the same on every replay of
/// a seeded game (#402).
#[must_use]
pub fn progress_fingerprint(state: &GameState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    state.turn_number.hash(&mut h);
    state.step.hash(&mut h);
    state.stack.len().hash(&mut h);
    for p in &state.players {
        p.life.hash(&mut h);
        p.land_plays_remaining.hash(&mut h);
        p.lost.hash(&mut h);
        p.library_order.len().hash(&mut h);
        // The graveyard is a public zone and its order is part of what is
        // on the table (CR 400.2 — any player may examine it); a mill or a
        // dredge that reorders it moved something. The pool is hashed by
        // colour, not by total: {R} spent and {G} floating is not the same
        // board as {R} floating and {G} spent.
        p.graveyard_order.hash(&mut h);
        p.mana_pool.mana.hash(&mut h);
    }
    // Sorted by id: `objects` is a map, and its iteration order is not the
    // game's (see #402 for what reading a map's order as an order costs).
    let mut objects: Vec<_> = state.objects.values().collect();
    objects.sort_by_key(|o| o.id);
    for o in &objects {
        o.id.hash(&mut h);
        o.zone.hash(&mut h);
        o.controller.hash(&mut h);
        o.tapped.hash(&mut h);
        o.summoning_sick.hash(&mut h);
        o.damage_marked.hash(&mut h);
        // What the permanent currently is, rather than which object it is.
        // A transform changes every one of these at once and used to change
        // none of them here (#560); a counter, an equip and a pump each
        // change one.
        o.name.hash(&mut h);
        o.is_transformed.hash(&mut h);
        o.counters.hash(&mut h);
        o.attached_to.hash(&mut h);
        o.attached_to_player.hash(&mut h);
        o.power.hash(&mut h);
        o.toughness.hash(&mut h);
        o.keywords.hash(&mut h);
        o.card_types.hash(&mut h);
        o.subtypes.hash(&mut h);
        o.regeneration_shields.hash(&mut h);
    }
    // Combat is state a decision can move without moving anything above it.
    // Announcing a damage assignment order (CR 509.2) touches nothing but
    // `combat`, and the engine re-raises that prompt once per blocker still
    // to be placed — so a board with two big gang blocks produced a hundred
    // legitimate, accepted, progressing decisions that every one of the
    // fields above read as identical, and the run was killed at turn 29
    // with a diagnosis that was the opposite of what happened (#509).
    //
    // All of it, not just the orders: every repeated prompt that lives in
    // `combat` alone has the same hole. The maps are ordered, so the hash
    // is the same on every replay of a seeded game (#402).
    state.combat.is_some().hash(&mut h);
    if let Some(combat) = &state.combat {
        for (attacker, defender) in &combat.attackers {
            attacker.hash(&mut h);
            defender.hash(&mut h);
        }
        for (attacker, walker) in &combat.planeswalker_defenders {
            attacker.hash(&mut h);
            walker.hash(&mut h);
        }
        for (attacker, blockers) in &combat.blocker_assignments {
            attacker.hash(&mut h);
            blockers.hash(&mut h);
        }
        for (attacker, order) in &combat.damage_assignment_order {
            attacker.hash(&mut h);
            order.hash(&mut h);
        }
        for attacker in &combat.blocked_attackers {
            attacker.hash(&mut h);
        }
        for creature in &combat.dealt_first_strike {
            creature.hash(&mut h);
        }
        combat.any_attackers_declared.hash(&mut h);
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

/// The move a runner sends to stop a game that will not stop itself.
///
/// It is *sent*, never looked up. CR 104.3a lets a player concede at any
/// time and [`LegalActions::permits`](mtg_engine::engine::LegalActions::permits)
/// admits it at any prompt, but `engine::legal_actions` only ever *lists*
/// it on the normal-priority path: at a mulligan, a discard, a declaration
/// or any resolution choice the function returns early with the answers to
/// that prompt alone, and for a set prompt that list is empty. So a runner
/// that reaches for `legal.actions.iter().position(|a| ... Concede)` finds
/// the move only when the stall happens to land on priority, and does
/// nothing at every other decision point — which is where a spinning seat
/// usually is. `mtg-draft-runner` had the stall forfeit *and* its
/// 50,000-action ceiling behind that lookup, so a game stuck at a prompt
/// had no termination condition at all: the standings said the game was
/// forfeit while the loop went on asking the seat (issue #559).
///
/// The engine accepts it there — that is the part that made the defect a
/// pure runner bug. `mtg-player/tests/stall_forfeit.rs` holds the contract.
#[must_use]
pub fn forfeit_move() -> mtg_engine::actions::Action {
    mtg_engine::actions::Action::Concede
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
    use mtg_engine::ids::ObjectId;

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

    /// Issue #509: announcing a damage assignment order (CR 509.2) moves
    /// nothing outside `state.combat`, and the engine raises that prompt
    /// once per blocker still to be placed. A token-flood board therefore
    /// produced one long run of legitimate, accepted, progressing decisions
    /// that the fingerprint scored as "changed nothing" — 91 placements
    /// across two gang-blocked attackers plus the 11 already made when the
    /// hundredth was reached, and the run was killed at turn 29 with a
    /// diagnosis that was the opposite of what had happened.
    ///
    /// Far more than `STALLED_DECISIONS` of them here, so that a fingerprint
    /// blind to combat cannot pass this by accident.
    #[test]
    fn placing_blockers_in_the_damage_assignment_order_is_progress() {
        let (mut state, _registry) = game();
        let mut combat = mtg_engine::state::CombatState {
            any_attackers_declared: true,
            ..Default::default()
        };
        let gangs: Vec<(ObjectId, Vec<ObjectId>)> = vec![
            (ObjectId(1), (100..160).map(ObjectId).collect()),
            (ObjectId(2), (200..260).map(ObjectId).collect()),
        ];
        for (attacker, blockers) in &gangs {
            combat.attackers.insert(*attacker, PlayerId(1));
            combat.blocked_attackers.insert(*attacker);
            combat.blocker_assignments.insert(*attacker, blockers.clone());
        }
        state.combat = Some(combat);

        let mut watchdog = ProgressWatchdog::new();
        let mut placed = 0;
        for (attacker, blockers) in &gangs {
            for blocker in blockers {
                assert!(!watchdog.observe(&state),
                    "killed a game that was moving, after {placed} placements");
                state.combat.as_mut().expect("combat")
                    .damage_assignment_order.entry(*attacker).or_default().push(*blocker);
                placed += 1;
            }
        }
        assert!(placed > STALLED_DECISIONS as usize, "{placed} is not enough to prove it");
        assert_eq!(watchdog.stalled_decisions(), 0);

        // And the watchdog still does its job in combat: a seat that really
        // is answering the same unusable way, with the same board and the
        // same combat, still trips.
        for _ in 0..STALLED_DECISIONS {
            watchdog.observe(&state);
        }
        assert!(watchdog.observe(&state), "the watchdog stopped watching");
    }

    /// Everything a player can look at and see is in the fingerprint.
    ///
    /// The list is a table rather than a test each, because these are one
    /// property of one computation: a board change is a change. It grew
    /// because two of them were missing — combat (#509, tested above) and
    /// then the permanent's own characteristics, so a werewolf flipping
    /// scored as "nothing moved" while the watchdog counted toward killing
    /// the game (#560).
    #[test]
    fn a_board_change_a_player_can_see_changes_the_fingerprint() {
        use mtg_engine::types::{CounterType, Keyword, ManaType};

        // The lowest-numbered object, so the pick is the same every run.
        let (base, _registry) = game();
        let subject = base.objects_in_id_order().first().expect("a card exists").id;

        let moves: Vec<(&str, fn(&mut GameState, ObjectId))> = vec![
            ("a werewolf flipped", |s, id| {
                let o = s.get_object_mut(id).expect("subject");
                o.is_transformed = !o.is_transformed;
                o.name = format!("{} (back)", o.name);
            }),
            ("a +1/+1 counter went on", |s, id| {
                s.get_object_mut(id).expect("subject")
                    .counters.insert(CounterType::PlusOnePlusOne, 1);
            }),
            ("an equipment was attached", |s, id| {
                s.get_object_mut(id).expect("subject").attached_to = Some(ObjectId(9999));
            }),
            ("a curse was attached to a player", |s, id| {
                s.get_object_mut(id).expect("subject").attached_to_player = Some(PlayerId(1));
            }),
            ("a creature was pumped", |s, id| {
                s.get_object_mut(id).expect("subject").power = Some(7);
            }),
            ("a creature gained flying", |s, id| {
                s.get_object_mut(id).expect("subject").keywords.push(Keyword::Flying);
            }),
            ("a permanent became an artifact", |s, id| {
                s.get_object_mut(id).expect("subject")
                    .card_types.push(mtg_engine::types::CardType::Artifact);
            }),
            ("a creature became a Vampire", |s, id| {
                s.get_object_mut(id).expect("subject").subtypes.push("Vampire".into());
            }),
            ("a regeneration shield was made", |s, id| {
                s.get_object_mut(id).expect("subject").regeneration_shields += 1;
            }),
            ("the graveyard was reordered", |s, _id| {
                s.players[0].graveyard_order.push(ObjectId(9998));
            }),
            ("the floating mana changed colour", |s, _id| {
                // Same total as the line below it on purpose: a pool
                // hashed by its total cannot tell these apart.
                s.players[0].mana_pool.add(ManaType::Red, 1);
            }),
        ];

        let before = progress_fingerprint(&base);
        for (what, apply) in moves {
            let mut state = base.clone();
            apply(&mut state, subject);
            assert_ne!(
                before,
                progress_fingerprint(&state),
                "{what}: the board moved and the fingerprint did not, so a game doing this \
                 over and over counts as stalled and is killed for making progress"
            );
        }
    }

    /// And the other direction, which is what makes the watchdog work at
    /// all: the decision in flight is deliberately invisible here.
    ///
    /// A seat that spins is *changing* these — the engine offers a cast,
    /// the seat names no target, the cast is cancelled and offered again
    /// (#462) — so a fingerprint that noticed them would never fire. Adding
    /// a field to the hash is cheap and this is the cost; the two tests
    /// together say where the line is.
    #[test]
    fn the_decision_in_flight_does_not_change_the_fingerprint() {
        let (base, _registry) = game();
        let before = progress_fingerprint(&base);

        let moves: Vec<(&str, fn(&mut GameState))> = vec![
            ("priority passed back", |s| {
                s.priority_player = Some(PlayerId(1));
            }),
            ("a pass was counted", |s| {
                s.consecutive_passes += 1;
            }),
            ("the trigger cursor moved", |s| {
                s.trigger_event_index += 1;
            }),
            ("the library was shuffled", |s| {
                s.players[0].library_order.reverse();
            }),
        ];

        for (what, apply) in moves {
            let mut state = base.clone();
            apply(&mut state);
            assert_eq!(
                before,
                progress_fingerprint(&state),
                "{what}: this is what moves while a seat spins, so counting it as progress \
                 means the watchdog never fires and an unusable answer repeats forever"
            );
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

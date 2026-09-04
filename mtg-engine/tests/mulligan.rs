//! Tests for the London mulligan opening-hand phase.
//!
//! Covers: initial hand deal, keep on first hand (no bottoming), mulligan
//! once and bottom one card, mulligan to the cap (3 mulls → forced keep →
//! bottom 3), library size and contents after each step, and that the
//! mulligan count is per-player.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::state::{AwaitingAction, GameState, LONDON_MULLIGAN_CAP};
use mtg_engine::types::*;

/// Build a minimal 40-card decklist for each player so that `setup_game`
/// has enough cards to deal seven and still leave a library.
fn test_decklist() -> Decklist {
    Decklist {
        entries: vec![
            ("Forest".into(), 20),
            ("Grizzly Bears".into(), 20),
        ],
    }
}

fn fresh_game() -> (GameState, CardRegistry) {
    let registry = CardRegistry::with_all_cards();
    let config = GameConfig {
        player_names: vec!["P0".into(), "P1".into()],
        decklists: vec![test_decklist(), test_decklist()],
        starting_life: 20,
        starting_player: None,
        rng_seed: Some(mtg_engine::state::default_rng_seed()),
    };
    let state = engine::setup_game(&config, &registry);
    (state, registry)
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    state.objects_in_zone(Zone::Hand, player).iter().map(|o| o.id).collect()
}

fn library_len(state: &GameState, player: PlayerId) -> usize {
    state.get_player(player).library_order.len()
}

// ── Initial state ───────────────────────────────────────────────────

#[test]
fn setup_game_enters_mulligan_phase() {
    let (state, _reg) = fresh_game();
    assert!(engine::in_mulligan_phase(&state));
    assert!(matches!(state.awaiting_action,
        Some(AwaitingAction::MulliganDecision { player }) if player == P0));
    assert_eq!(hand_ids(&state, P0).len(), 7);
    assert_eq!(hand_ids(&state, P1).len(), 7);
    assert_eq!(library_len(&state, P0), 40 - 7);
    assert_eq!(library_len(&state, P1), 40 - 7);
    assert_eq!(state.get_player(P0).mulligan_count, 0);
    assert_eq!(state.get_player(P1).mulligan_count, 0);
}

#[test]
fn legal_actions_initial_mulligan_decision_offers_keep_and_mull() {
    let (state, reg) = fresh_game();
    let legal = engine::legal_actions(&state, &reg);
    // Order: MulliganKeep then MulliganMull (since count < cap).
    assert_eq!(legal.actions.len(), 2);
    assert!(matches!(legal.actions[0], Action::MulliganKeep));
    assert!(matches!(legal.actions[1], Action::MulliganMull));
}

// ── Keep path ───────────────────────────────────────────────────────

#[test]
fn both_players_keep_first_hand_no_bottoming() {
    let (mut state, reg) = fresh_game();
    let p0_hand = hand_ids(&state, P0);
    let p1_hand = hand_ids(&state, P1);
    let p0_lib_before = library_len(&state, P0);
    let p1_lib_before = library_len(&state, P1);

    engine::run_mulligan_phase(&mut state, &reg, |_, _, legal| {
        // Pick MulliganKeep whenever available.
        for a in &legal.actions {
            if matches!(a, Action::MulliganKeep) { return a.clone(); }
        }
        unreachable!("MulliganKeep should always be a legal action in this test");
    });

    // Mulligan phase should be over.
    assert!(!engine::in_mulligan_phase(&state));
    assert_eq!(state.get_player(P0).mulligan_count, 0);
    assert_eq!(state.get_player(P1).mulligan_count, 0);
    // Hands unchanged (same ObjectIds), libraries unchanged.
    assert_eq!(hand_ids(&state, P0), p0_hand);
    assert_eq!(hand_ids(&state, P1), p1_hand);
    assert_eq!(library_len(&state, P0), p0_lib_before);
    assert_eq!(library_len(&state, P1), p1_lib_before);
}

// ── Single mulligan path ────────────────────────────────────────────

/// P0 mulls once, then keeps (and bottoms one); P1 keeps first hand.
/// Verifies: `mulligan_count==1` for P0, hand size stays 7 during the
/// bottoming prompt, library size drops by 1 after bottoming, the
/// bottomed card lands at the bottom of the library.
#[test]
fn mull_once_then_bottom_one() {
    let (mut state, reg) = fresh_game();
    let mut p0_mull_done = false;

    engine::run_mulligan_phase(&mut state, &reg, |gs, player, _legal| {
        if player == P0 {
            match &gs.awaiting_action {
                Some(AwaitingAction::MulliganDecision { .. }) => {
                    if !p0_mull_done {
                        p0_mull_done = true;
                        return Action::MulliganMull;
                    }
                    return Action::MulliganKeep;
                }
                Some(AwaitingAction::BottomAfterMulligan { count, .. }) => {
                    assert_eq!(*count, 1);
                    let hand = hand_ids(gs, P0);
                    return Action::BottomCards { cards: vec![hand[0]] };
                }
                _ => {}
            }
        }
        // P1 keeps unconditionally.
        Action::MulliganKeep
    });

    assert!(!engine::in_mulligan_phase(&state));
    assert_eq!(state.get_player(P0).mulligan_count, 1);
    assert_eq!(state.get_player(P1).mulligan_count, 0);
    // P0 hand is 6 (drew 7, bottomed 1).
    assert_eq!(hand_ids(&state, P0).len(), 6);
    // P0 library is 40 - 6 = 34 (7 drawn, 1 bottomed back).
    assert_eq!(library_len(&state, P0), 34);
    // The bottomed card is the last entry in library_order.
    let bottom_card = *state.get_player(P0).library_order.last().unwrap();
    // And it's not in the hand anymore.
    assert!(!hand_ids(&state, P0).contains(&bottom_card));
}

// ── Mull-to-cap path ────────────────────────────────────────────────

/// P0 mulls to the cap (3 mulls) and is then forced to keep. They must
/// bottom 3. Legal actions on the 4th prompt should NOT include mull.
#[test]
fn mull_to_cap_forces_keep_and_bottoms_three() {
    let (mut state, reg) = fresh_game();
    let mut p0_mulls_taken = 0;
    let mut p0_saw_forced_keep = false;

    engine::run_mulligan_phase(&mut state, &reg, |gs, player, legal| {
        if player == P0 {
            match &gs.awaiting_action {
                Some(AwaitingAction::MulliganDecision { .. }) => {
                    let has_mull = legal.actions.iter()
                        .any(|a| matches!(a, Action::MulliganMull));
                    if p0_mulls_taken < LONDON_MULLIGAN_CAP as usize {
                        assert!(has_mull, "mull should be legal at count {p0_mulls_taken}");
                        p0_mulls_taken += 1;
                        return Action::MulliganMull;
                    }
                    // At cap: mulligan should NOT be offered.
                    assert!(!has_mull, "mull should not be legal at cap");
                    p0_saw_forced_keep = true;
                    return Action::MulliganKeep;
                }
                Some(AwaitingAction::BottomAfterMulligan { count, .. }) => {
                    assert_eq!(*count, LONDON_MULLIGAN_CAP as usize);
                    let hand = hand_ids(gs, P0);
                    return Action::BottomCards {
                        cards: hand.iter().take(*count).copied().collect(),
                    };
                }
                _ => {}
            }
        }
        // P1 keeps unconditionally.
        Action::MulliganKeep
    });

    assert!(p0_saw_forced_keep, "P0 should have seen the forced-keep prompt");
    assert!(!engine::in_mulligan_phase(&state));
    assert_eq!(state.get_player(P0).mulligan_count, LONDON_MULLIGAN_CAP);
    // Mull-to-4: 7 drawn, 3 bottomed → hand size 4.
    assert_eq!(hand_ids(&state, P0).len(), 4);
    // Library should contain (40 - 4) = 36 cards.
    assert_eq!(library_len(&state, P0), 36);
}

// ── Per-player independence ────────────────────────────────────────

#[test]
fn per_player_mulligan_counts_are_independent() {
    let (mut state, reg) = fresh_game();
    let mut p0_mulls = 0;
    let mut p1_mulls = 0;

    engine::run_mulligan_phase(&mut state, &reg, |gs, player, _legal| {
        match &gs.awaiting_action {
            Some(AwaitingAction::MulliganDecision { .. }) => {
                if player == P0 && p0_mulls < 2 {
                    p0_mulls += 1;
                    return Action::MulliganMull;
                }
                if player == P1 && p1_mulls < 1 {
                    p1_mulls += 1;
                    return Action::MulliganMull;
                }
                Action::MulliganKeep
            }
            Some(AwaitingAction::BottomAfterMulligan { count, .. }) => {
                let hand = hand_ids(gs, player);
                Action::BottomCards {
                    cards: hand.iter().take(*count).copied().collect(),
                }
            }
            _ => unreachable!("run_mulligan_phase only invokes the callback for mulligan/bottom decisions"),
        }
    });

    assert_eq!(state.get_player(P0).mulligan_count, 2);
    assert_eq!(state.get_player(P1).mulligan_count, 1);
    // Hand sizes: P0 = 7 - 2 = 5, P1 = 7 - 1 = 6.
    assert_eq!(hand_ids(&state, P0).len(), 5);
    assert_eq!(hand_ids(&state, P1).len(), 6);
    // Library sizes: 40 - hand.
    assert_eq!(library_len(&state, P0), 35);
    assert_eq!(library_len(&state, P1), 34);
}

// ── Round alternation ──────────────────────────────────────────────

/// Verify the keep/mull sub-phase alternates between players within a
/// round rather than draining one player's decisions before moving to
/// the next. The active player decides first within each round; the
/// non-active player decides next, with knowledge of the active
/// player's current-round decision; then any player who mulled gets
/// asked again in the next round.
#[test]
fn keep_mull_decisions_alternate_round_by_round() {
    let (mut state, reg) = fresh_game();
    // Record the (round, player) sequence in the order asked.
    let mut decisions: Vec<(u32, PlayerId)> = Vec::new();
    let mut p0_mulls = 0;
    let mut p1_mulls = 0;

    engine::run_mulligan_phase(&mut state, &reg, |gs, player, _legal| {
        match &gs.awaiting_action {
            Some(AwaitingAction::MulliganDecision { .. }) => {
                let round = u32::try_from(decisions.iter().filter(|(_, p)| *p == player).count()).unwrap_or(0) + 1;
                decisions.push((round, player));
                // P0: mull rounds 1 and 2, keep round 3.
                // P1: mull round 1, keep round 2.
                if player == P0 && p0_mulls < 2 {
                    p0_mulls += 1;
                    return Action::MulliganMull;
                }
                if player == P1 && p1_mulls < 1 {
                    p1_mulls += 1;
                    return Action::MulliganMull;
                }
                Action::MulliganKeep
            }
            Some(AwaitingAction::BottomAfterMulligan { count, .. }) => {
                let hand = hand_ids(gs, player);
                Action::BottomCards {
                    cards: hand.iter().take(*count).copied().collect(),
                }
            }
            _ => unreachable!(),
        }
    });

    // Expected interleaving (active player P0 decides first per round):
    //   Round 1: P0 (mull), P1 (mull)
    //   Round 2: P0 (mull), P1 (keep)
    //   Round 3: P0 (keep)            ← P1 already kept, skipped
    let expected = vec![
        (1, P0), (1, P1),
        (2, P0), (2, P1),
        (3, P0),
    ];
    assert_eq!(decisions, expected,
        "decisions should alternate between players within each round, not drain one player at a time");
    assert_eq!(state.get_player(P0).mulligan_count, 2);
    assert_eq!(state.get_player(P1).mulligan_count, 1);
}

// ── Library integrity ──────────────────────────────────────────────

/// After mulligans and bottoming, every card the player owns should still
/// exist somewhere — nothing is lost or duplicated.
#[test]
fn library_and_hand_contain_all_dealt_cards() {
    let (mut state, reg) = fresh_game();

    engine::run_mulligan_phase(&mut state, &reg, |gs, player, _legal| {
        match &gs.awaiting_action {
            Some(AwaitingAction::MulliganDecision { .. }) => {
                // P0: mull once. P1: keep immediately.
                if player == P0 && gs.get_player(P0).mulligan_count == 0 {
                    Action::MulliganMull
                } else {
                    Action::MulliganKeep
                }
            }
            Some(AwaitingAction::BottomAfterMulligan { count, .. }) => {
                let hand = hand_ids(gs, player);
                Action::BottomCards {
                    cards: hand.iter().take(*count).copied().collect(),
                }
            }
            _ => unreachable!("run_mulligan_phase only invokes the callback for mulligan/bottom decisions"),
        }
    });

    // P0 should own exactly 40 cards across hand + library (graveyard and
    // exile should be empty at game start).
    for &pid in &[P0, P1] {
        let hand: std::collections::HashSet<_> = hand_ids(&state, pid).into_iter().collect();
        let lib: std::collections::HashSet<_> =
            state.get_player(pid).library_order.iter().copied().collect();
        assert_eq!(hand.intersection(&lib).count(), 0,
            "hand and library should not share objects");
        let total = hand.len() + lib.len();
        assert_eq!(total, 40, "p{} should own 40 cards total", pid.0);
    }
}

// ── Integration: full mini game drives through mulligan phase ─────

/// A short end-to-end game with a trivial scripted player that keeps the
/// first hand for both seats and then immediately concedes. This verifies
/// that the mulligan phase hands off cleanly to turn 1 and that the
/// callback pattern continues to work.
#[test]
fn integration_keep_then_concede_ends_game() {
    let (mut state, reg) = fresh_game();
    let mut saw_turn1 = false;

    engine::run_game_loop(&mut state, &reg, |gs, _player, legal| {
        if !engine::in_mulligan_phase(gs) {
            saw_turn1 = true;
            // Immediately concede from turn 1 to end the game.
            if legal.actions.iter().any(|a| matches!(a, Action::Concede)) {
                return Action::Concede;
            }
        }
        for a in &legal.actions {
            if matches!(a, Action::MulliganKeep) { return a.clone(); }
        }
        Action::PassPriority
    });

    assert!(saw_turn1, "Game loop should have reached turn 1 after mulligans");
    assert!(state.is_game_over());
}

/// The turn-1 banner names whoever is actually taking turn 1.
///
/// It was the literal "── Turn 1 (p0) ──", so a game p1 opened — an
/// `--on-the-play 2`, or simply losing the CR 103.1 roll — announced turn 1
/// as p0's in the `--log` file and, through the harness's turn-banner
/// rewrite, in the first prompt every LLM seat sees. The turn-1
/// `TurnStarted` event always carried the right player; only the line people
/// read did not.
#[test]
fn the_turn_1_banner_names_the_player_who_takes_turn_1() {
    let registry = CardRegistry::with_all_cards();

    for starting in [P0, P1] {
        let config = GameConfig {
            player_names: vec!["P0".into(), "P1".into()],
            decklists: vec![test_decklist(), test_decklist()],
            starting_life: 20,
            starting_player: Some(starting),
            rng_seed: Some(mtg_engine::state::default_rng_seed()),
        };
        let mut state = engine::setup_game(&config, &registry);

        // Drive the mulligan phase to completion: both seats keep.
        engine::run_game_loop(&mut state, &registry, |gs, _player, legal| {
            if !engine::in_mulligan_phase(gs)
                && legal.actions.iter().any(|a| matches!(a, Action::Concede))
            {
                return Action::Concede;
            }
            for a in &legal.actions {
                if matches!(a, Action::MulliganKeep) { return a.clone(); }
            }
            Action::PassPriority
        });

        let banner = state.game_log.iter()
            .find(|e| e.message.starts_with("── Turn 1 "))
            .map(|e| e.message.clone())
            .expect("turn 1 is announced");
        assert_eq!(banner, format!("── Turn 1 (p{}) ──", starting.0),
            "turn 1 belongs to the player on the play");
    }
}

/// Issue #112 (CR 103.1): the opening player is determined at random —
/// seeded, so a --seed game replays identically — instead of always p0.
/// An explicit starting_player (loser-chooses, --on-the-play) still wins.
#[test]
fn the_opening_player_is_seeded_random_not_always_p0() {
    let registry = CardRegistry::with_all_cards();
    let game_with = |starting: Option<PlayerId>, seed: u64| {
        let config = GameConfig {
            player_names: vec!["P0".into(), "P1".into()],
            decklists: vec![test_decklist(), test_decklist()],
            starting_life: 20,
            starting_player: starting,
            rng_seed: Some(seed),
        };
        engine::setup_game(&config, &registry).active_player
    };

    // Reproducible: the same seed always yields the same opening player.
    assert_eq!(game_with(None, 42), game_with(None, 42));

    // Random: across seeds, both seats win the roll sometimes.
    let winners: std::collections::HashSet<PlayerId> =
        (0u64..32).map(|s| game_with(None, s)).collect();
    assert_eq!(winners.len(), 2,
        "both seats should win the die roll across 32 seeds — always-p0 was \
         the issue #112 bias");

    // An explicit choice wins over the roll.
    assert_eq!(game_with(Some(PlayerId(1)), 42), PlayerId(1));
    assert_eq!(game_with(Some(PlayerId(0)), 42), PlayerId(0));
}

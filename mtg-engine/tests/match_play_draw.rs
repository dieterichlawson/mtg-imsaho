//! Regression tests for the MTG tournament play/draw rules exposed by
//! `engine::random_starting_player` and
//! `engine::next_starter_loser_plays`.

use mtg_engine::engine;
use mtg_engine::ids::PlayerId;

#[test]
fn random_starting_player_returns_in_range_over_many_flips() {
    // Drawing 1000 samples from a 2-player coin flip should hit both
    // values and never go out of range.
    let mut saw_0 = false;
    let mut saw_1 = false;
    for _ in 0..1000 {
        let p = engine::random_starting_player(2);
        match p.0 {
            0 => saw_0 = true,
            1 => saw_1 = true,
            other => panic!("random_starting_player returned out-of-range id {other}"),
        }
    }
    assert!(saw_0 && saw_1,
        "Fair coin flip over 1000 samples should hit both players");
}

#[test]
fn random_starting_player_single_player_game_is_deterministic() {
    for _ in 0..10 {
        assert_eq!(engine::random_starting_player(1), PlayerId(0));
    }
}

/// MTR 2.3: the loser of the previous game chooses who plays first, and always
/// elects to play. A drawn game has no loser, so the previous choice persists.
///
/// This was four tests over six cases; the whole rule is a truth table, so it
/// reads as one.
#[test]
fn the_loser_of_the_last_game_chooses_and_elects_to_play() {
    // (who started last game, who won — None for a draw, who starts next)
    const CASES: &[(u8, Option<u8>, u8)] = &[
        (0, Some(0), 1),  // starter won, so the other player chooses and plays
        (1, Some(1), 0),
        (0, Some(1), 0),  // the non-starter won, so the starter chooses and plays
        (1, Some(0), 1),
        (0, None, 0),     // a draw has no loser — the previous choice stands
        (1, None, 1),
    ];

    for &(started, won, expected) in CASES {
        let next = engine::next_starter_loser_plays(
            PlayerId(started),
            won.map(PlayerId),
            2,
        );
        assert_eq!(next, PlayerId(expected),
            "p{started} started and {} — p{expected} should start the next game",
            won.map_or("the game was drawn".to_string(), |w| format!("p{w} won")));
    }
}

#[test]
#[should_panic(expected = "only supports 2-player matches")]
fn next_starting_player_panics_for_multiplayer() {
    let _ = engine::next_starter_loser_plays(PlayerId(0), Some(PlayerId(0)), 3);
}

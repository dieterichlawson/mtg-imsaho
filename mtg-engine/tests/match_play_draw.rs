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
            other => panic!("random_starting_player returned out-of-range id {}", other),
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

#[test]
fn loser_chooses_after_p0_wins() {
    // p0 was on the play and won → p1 (the loser) chooses next game →
    // loser elects to play first → next starter is p1.
    let next = engine::next_starter_loser_plays(
        PlayerId(0),
        Some(PlayerId(0)),
        2,
    );
    assert_eq!(next, PlayerId(1));
}

#[test]
fn loser_chooses_after_p1_wins() {
    // p1 was on the play and won → p0 (the loser) chooses next game →
    // next starter is p0.
    let next = engine::next_starter_loser_plays(
        PlayerId(1),
        Some(PlayerId(1)),
        2,
    );
    assert_eq!(next, PlayerId(0));
}

#[test]
fn loser_chooses_when_non_starter_wins() {
    // p0 was on the play but p1 won → p0 (the loser) chooses next game.
    let next = engine::next_starter_loser_plays(
        PlayerId(0),
        Some(PlayerId(1)),
        2,
    );
    assert_eq!(next, PlayerId(0));

    // And the mirror case: p1 on the play, p0 won → p1 chooses next.
    let next = engine::next_starter_loser_plays(
        PlayerId(1),
        Some(PlayerId(0)),
        2,
    );
    assert_eq!(next, PlayerId(1));
}

#[test]
fn drawn_game_keeps_previous_starter() {
    // Per MTR §2.3, a drawn game has no loser and the previous pre-game
    // choice persists into the next game.
    let next = engine::next_starter_loser_plays(
        PlayerId(0),
        None,
        2,
    );
    assert_eq!(next, PlayerId(0));

    let next = engine::next_starter_loser_plays(
        PlayerId(1),
        None,
        2,
    );
    assert_eq!(next, PlayerId(1));
}

#[test]
#[should_panic(expected = "only supports 2-player matches")]
fn next_starting_player_panics_for_multiplayer() {
    engine::next_starter_loser_plays(PlayerId(0), Some(PlayerId(0)), 3);
}

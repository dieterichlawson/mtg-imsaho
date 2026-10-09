//! The game-over headline is one builder for both runners (#743).
//!
//! The draft runner's match loop wrote its own, "Game over! Seat 1 wins."
//! with no loss reason and a draw ending in a full stop, so the hosted
//! table's page never said a seat conceded or forfeited and could not tell
//! YOU WIN from OPPONENT WINS: the page reads `Game over! p<N>` and
//! `It's a draw!`.

use mtg_engine::events::LossReason;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_player::game_over_headline;

fn table_label(id: PlayerId) -> String {
    format!("p{} (Seat {})", id.0, [3, 1][id.0 as usize])
}

#[test]
fn a_concede_and_a_forfeit_are_said_and_the_winner_is_named_by_p_number() {
    for (reason, said) in [(LossReason::Conceded, "conceded"), (LossReason::Forfeited, "forfeited")] {
        let mut state = GameState::new(2);
        state.player_loses(PlayerId(0), reason);
        state.result = Some(mtg_engine::state::GameResult::Winner(PlayerId(1)));
        let line = game_over_headline(&state, table_label).expect("the game has a result");
        assert!(line.starts_with("Game over! p1 (Seat 1) wins!"), "the page reads `Game over! p<N>`: {line}");
        assert!(line.contains("p0 (Seat 3)") && line.contains(said), "how the loser lost: {line}");
    }
}

#[test]
fn a_draw_is_the_pages_draw_and_no_result_is_none() {
    let state = GameState::new(2);
    assert_eq!(game_over_headline(&state, table_label), None);
    let mut state = GameState::new(2);
    state.result = Some(mtg_engine::state::GameResult::Draw);
    assert!(game_over_headline(&state, table_label).unwrap().starts_with("Game over! It's a draw!"));
}

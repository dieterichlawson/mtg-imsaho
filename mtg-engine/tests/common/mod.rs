//! Shared test helpers.

use mtg_engine::ids::{CardId, ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

pub const P0: PlayerId = PlayerId(0);
pub const P1: PlayerId = PlayerId(1);

/// Set up a game state at a specific step with P0 as active and having priority.
pub fn game_at_step(step: Step, active: PlayerId) -> GameState {
    let mut state = GameState::new(2);
    state.step = step;
    state.active_player = active;
    state.priority_player = Some(active);
    state.is_first_turn = false;
    state.players[0].life = 20;
    state.players[1].life = 20;
    state
}

/// Place a creature on the battlefield that is ready to act (no summoning sickness).
pub fn ready_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    let id = state.create_object(CardId(99), owner, Zone::Battlefield, Some(power), Some(toughness));
    state.get_object_mut(id).unwrap().summoning_sick = false;
    id
}

/// Place a creature on the battlefield with summoning sickness.
pub fn sick_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    state.create_object(CardId(99), owner, Zone::Battlefield, Some(power), Some(toughness))
}

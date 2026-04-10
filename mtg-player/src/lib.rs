pub mod random;
pub mod cli;
pub mod llm;
pub mod scripted;
pub mod game_log;

use mtg_engine::view::GameView;
use mtg_engine::actions::Action;
use mtg_engine::engine::LegalActions;

/// The Player trait: given a view of the game and legal actions, pick one.
pub trait Player {
    fn name(&self) -> &str;

    /// Choose an action given the full legal actions structure.
    /// For most players, only `legal.actions` matters. The CLI uses
    /// `legal.castable_spells` for interactive target selection.
    fn choose_action(&mut self, view: &GameView, legal: &LegalActions) -> Action;
}

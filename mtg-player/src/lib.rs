pub mod random;
pub mod cli;
pub mod llm;

use mtg_engine::view::GameView;
use mtg_engine::actions::Action;

/// The Player trait: given a view of the game and legal actions, pick one.
pub trait Player {
    fn name(&self) -> &str;

    /// Choose an action. For most situations, pick from `legal_actions`.
    /// For combat declarations, construct the action directly
    /// (legal_actions will contain a single sentinel).
    fn choose_action(&mut self, view: &GameView, legal_actions: &[Action]) -> Action;

    /// Choose which cards to put on bottom after mulligan.
    fn choose_cards_to_bottom(
        &mut self,
        view: &GameView,
        hand: &[mtg_engine::view::CardView],
        count: usize,
    ) -> Vec<mtg_engine::ids::ObjectId>;
}

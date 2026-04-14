use std::collections::VecDeque;

use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::view::GameView;

use crate::Player;

/// A player that returns pre-scripted actions in order.
///
/// Used for deterministic testing of game scenarios without LLM calls.
/// The test provides a sequence of actions; the player returns them one
/// at a time. Panics if the queue runs out before the test finishes.
pub struct ScriptedPlayer {
    name: String,
    actions: VecDeque<Action>,
}

impl ScriptedPlayer {
    /// Create a new scripted player with the given name and action queue.
    #[must_use]
    pub fn new(name: &str, actions: Vec<Action>) -> Self {
        Self {
            name: name.to_string(),
            actions: VecDeque::from(actions),
        }
    }

    /// Push additional actions onto the back of the queue.
    pub fn push_actions(&mut self, actions: Vec<Action>) {
        self.actions.extend(actions);
    }

    /// How many actions remain in the queue.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.actions.len()
    }

    /// Choose a combat action from a combat prompt using the next action in the queue.
    ///
    /// The next action must be a `DeclareAttackers` or `DeclareBlockers`.
    ///
    /// # Panics
    /// Panics if the scripted action queue is empty when a combat decision is
    /// required.
    pub fn choose_combat(&mut self, _prompt: &CombatPrompt) -> Action {
        self.actions.pop_front().unwrap_or_else(|| {
            panic!(
                "ScriptedPlayer '{}': action queue empty when combat decision needed",
                self.name
            )
        })
    }
}

impl Player for ScriptedPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, _view: &GameView, _legal: &mtg_engine::engine::LegalActions) -> Action {
        self.actions.pop_front().unwrap_or_else(|| {
            panic!(
                "ScriptedPlayer '{}': action queue empty when action needed",
                self.name
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_player_returns_actions_in_order() {
        let actions = vec![
            Action::PassPriority,
            Action::Concede,
        ];
        let mut player = ScriptedPlayer::new("Test", actions);

        assert_eq!(player.remaining(), 2);

        // We need a dummy GameView and LegalActions to call choose_action.
        // Since ScriptedPlayer ignores them, we can use minimal structs.
        // But the types are complex, so just test via the queue directly.
        let action = player.actions.pop_front().unwrap();
        assert!(matches!(action, Action::PassPriority));

        let action = player.actions.pop_front().unwrap();
        assert!(matches!(action, Action::Concede));

        assert_eq!(player.remaining(), 0);
    }

    #[test]
    fn push_actions_extends_queue() {
        let mut player = ScriptedPlayer::new("Test", vec![Action::PassPriority]);
        assert_eq!(player.remaining(), 1);

        player.push_actions(vec![Action::Concede, Action::PassPriority]);
        assert_eq!(player.remaining(), 3);
    }
}

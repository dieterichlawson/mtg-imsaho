use rand::Rng;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::ids::ObjectId;
use mtg_engine::view::GameView;

use crate::Player;

/// A player that picks randomly from legal actions.
pub struct RandomPlayer {
    name: String,
}

impl RandomPlayer {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl Player for RandomPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, _view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        let legal_actions = &legal.actions;
        // Filter out Concede.
        let non_concede: Vec<usize> = legal_actions.iter().enumerate()
            .filter(|(_, a)| !matches!(a, Action::Concede))
            .map(|(i, _)| i)
            .collect();

        let candidates = if non_concede.is_empty() {
            (0..legal_actions.len()).collect::<Vec<_>>()
        } else {
            non_concede
        };

        if candidates.len() == 1 {
            return legal_actions[candidates[0]].clone();
        }
        let mut rng = rand::thread_rng();
        legal_actions[candidates[rng.gen_range(0..candidates.len())]].clone()
    }

    fn choose_cards_to_bottom(
        &mut self,
        _view: &GameView,
        hand: &[mtg_engine::view::CardView],
        count: usize,
    ) -> Vec<ObjectId> {
        hand.iter().take(count).map(|c| c.object_id).collect()
    }
}

impl RandomPlayer {
    /// Choose a random combat action from a combat prompt.
    pub fn choose_combat(&mut self, prompt: &CombatPrompt) -> Action {
        let mut rng = rand::thread_rng();
        match prompt {
            CombatPrompt::ChooseAttackers { eligible, defending_player } => {
                // Each eligible creature has a 50% chance of attacking.
                let attackers: Vec<_> = eligible.iter()
                    .filter(|_| rng.gen_bool(0.5))
                    .map(|&id| (id, *defending_player))
                    .collect();
                Action::DeclareAttackers { attackers }
            }
            CombatPrompt::ChooseBlockers { eligible_blockers, attackers } => {
                if attackers.is_empty() {
                    return Action::DeclareBlockers { assignments: vec![] };
                }
                // Each eligible blocker has a 50% chance of blocking a random attacker.
                let mut assignments = Vec::new();
                for &blocker in eligible_blockers {
                    if rng.gen_bool(0.5) {
                        let attacker = attackers[rng.gen_range(0..attackers.len())];
                        assignments.push((blocker, attacker));
                    }
                }
                Action::DeclareBlockers { assignments }
            }
        }
    }
}

use rand::Rng;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::view::GameView;

use crate::Player;

/// A player that picks randomly from legal actions.
pub struct RandomPlayer {
    name: String,
}

impl RandomPlayer {
    #[must_use]
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

        // X-cost funding: no enumerated actions to pick from. Default to
        // tapping everything (max X), which is rarely optimal but lets
        // RandomPlayer-driven tests make forward progress through X-cost
        // spells/abilities without requiring smart choices.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. }) =
            legal.resolution_prompt.as_ref()
        {
            use mtg_engine::actions::ResolvedChoice;
            use mtg_engine::funding::FundingResponse;
            let mut response = FundingResponse::default();
            for (mt, amt) in &options.pool {
                if *amt > 0 {
                    response.pool.insert(*mt, *amt);
                }
            }
            for g in &options.groups {
                response.taps.insert(g.name.clone(), g.max_contribution());
            }
            return Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
        }

        // Deterministic mulligan policy: always keep the first hand, never
        // mulligan. For the bottom sub-phase (only reached via a forced keep
        // at the cap or if this player had previously mulliganed), pick the
        // first enumerated combination. This avoids introducing opening-hand
        // RNG beyond the deal itself — desirable for experiments that want
        // to measure deck quality and piloting, not mulligan variance.
        if let Some(keep_idx) = legal_actions.iter().position(|a| matches!(a, Action::MulliganKeep)) {
            return legal_actions[keep_idx].clone();
        }
        if matches!(legal_actions.first(), Some(Action::BottomCards { .. })) {
            return legal_actions[0].clone();
        }

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
}

impl RandomPlayer {
    /// Choose a random combat action from a combat prompt.
    pub fn choose_combat(&mut self, prompt: &CombatPrompt) -> Action {
        let mut rng = rand::thread_rng();
        match prompt {
            CombatPrompt::ChooseAttackers { eligible, defending_player, .. } => {
                // Each eligible creature has a 50% chance of attacking.
                let attackers: Vec<_> = eligible.iter()
                    .filter(|_| rng.gen_bool(0.5))
                    .map(|&id| (id, *defending_player))
                    .collect();
                Action::DeclareAttackers { attackers }
            }
            CombatPrompt::ChooseBlockers { eligible_blockers, attackers, .. } => {
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

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::view::GameView;

use crate::Player;

/// A player that picks randomly from legal actions.
pub struct RandomPlayer {
    name: String,
    rng: StdRng,
}

impl RandomPlayer {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), rng: StdRng::from_entropy() }
    }

    /// A player whose choices replay identically for the same seed. Pair with
    /// `GameConfig::rng_seed` to make a whole game deterministic.
    #[must_use]
    pub fn with_seed(name: &str, seed: u64) -> Self {
        Self { name: name.to_string(), rng: StdRng::seed_from_u64(seed) }
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

        // Exile-from-graveyard additional cost: pick the minimum size subset
        // (which is 0 for Harvest Pyre, n for Stitched Drake / Skaab Ruinator).
        // Matches the RandomPlayer convention of "minimal action, always valid."
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
            options, min, ..
        }) = legal.resolution_prompt.as_ref()
        {
            use mtg_engine::actions::ResolvedChoice;
            let chosen: Vec<mtg_engine::ids::ObjectId> = options.iter().take(*min).copied().collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(chosen) };
        }

        // An "up to N" target slot: no enumerated actions — the subsets are
        // exponential in the board — so the count is rolled and the targets
        // taken in order, no target twice (CR 601.2c).
        //
        // NOT the minimum, unlike the costs above. `min` is zero for every
        // "up to N" slot in the pool, so taking it would mean this seat
        // never casts Feeling of Dread at a creature, never taps anything
        // with it, and never resolves the half of those cards that does
        // something — and this seat is what the invariant fuzzer plays.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseTargetSet {
            options, min, max, ..
        }) = legal.resolution_prompt.as_ref()
        {
            use mtg_engine::actions::ResolvedChoice;
            use rand::seq::SliceRandom;
            let how_many = if max > min { self.rng.gen_range(*min..=*max) } else { *min };
            // A random subset, not the first `how_many`: taking them in
            // order would mean the last creature on a wide board is never
            // targeted at all.
            let chosen: Vec<mtg_engine::actions::Target> =
                options.choose_multiple(&mut self.rng, how_many).cloned().collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(chosen) };
        }

        // A set of objects chosen while something resolves (Curse of
        // Oblivion's two cards). Roll the count and take a random subset,
        // for the same reason the target set above does: the minimum is
        // sometimes zero, and a seat that always answers with nothing is a
        // seat that never exercises the effect.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseObjectSet {
            options, min, max, ..
        }) = legal.resolution_prompt.as_ref()
        {
            use mtg_engine::actions::ResolvedChoice;
            use rand::seq::SliceRandom;
            let how_many = if max > min { self.rng.gen_range(*min..=*max) } else { *min };
            let chosen: Vec<mtg_engine::ids::ObjectId> =
                options.choose_multiple(&mut self.rng, how_many).copied().collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenObjectSet(chosen) };
        }

        // Pile division (Liliana of the Veil -6): no enumerated actions —
        // 2^N subsets don't fit in memory on a wide board. Flip a coin per
        // permanent, mirroring the 50% conventions used for combat.
        if let Some(mtg_engine::state::ResolutionChoiceKind::DividePermanentsIntoPiles {
            permanents, ..
        }) = legal.resolution_prompt.as_ref()
        {
            use mtg_engine::actions::ResolvedChoice;
            let chosen: Vec<mtg_engine::ids::ObjectId> = permanents.iter()
                .filter(|_| self.rng.gen_bool(0.5))
                .copied()
                .collect();
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenSubset(chosen) };
        }

        // A set of cards out of a list — the mulligan bottoming and the
        // cleanup discard. There are no enumerated actions to pick from:
        // the subsets are C(hand, n), which is a menu nobody can read and,
        // for this player, a list to index into for no benefit. Take the
        // first `min` in hand order, the same "minimal action, always
        // valid" convention as the exile cost above, and the same
        // deterministic opening hand as before: no mulligan RNG beyond the
        // deal itself.
        if let Some(prompt) = legal.set_prompt.as_ref() {
            let chosen: Vec<mtg_engine::ids::ObjectId> =
                prompt.options.iter().take(prompt.min).copied().collect();
            return prompt.answer(chosen);
        }

        // Deterministic mulligan policy: always keep the first hand, never
        // mulligan.
        if let Some(keep_idx) = legal_actions.iter().position(|a| matches!(a, Action::MulliganKeep)) {
            return legal_actions[keep_idx].clone();
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
        let pick = self.rng.gen_range(0..candidates.len());
        legal_actions[candidates[pick]].clone()
    }
}

impl RandomPlayer {
    /// Choose a random combat action from a combat prompt.
    pub fn choose_combat(&mut self, prompt: &CombatPrompt) -> Action {
        let rng = &mut self.rng;
        match prompt {
            CombatPrompt::ChooseAttackers {
                eligible, defending_player, defending_planeswalkers, ..
            } => {
                // Each eligible creature has a 50% chance of attacking, and
                // each attacker picks uniformly among the legal defenders:
                // the defending player, or any planeswalker they control
                // (CR 508.1a).
                //
                // Sending every attacker at the player was a hard-coded empty
                // `planeswalker_attacks`, which made the walker-combat path
                // unreachable for a random seat no matter how many games ran —
                // so every invariant over it (`planeswalker_defenders`, damage
                // routed to an attacked walker, the CR 702.19b trample
                // assignment measured in loyalty) was unfuzzable (issue #220).
                let mut attackers = Vec::new();
                let mut planeswalker_attacks = Vec::new();
                for &id in eligible {
                    if !rng.gen_bool(0.5) {
                        continue;
                    }
                    // Slot 0 is the player; slots 1.. are the walkers.
                    let slot = rng.gen_range(0..=defending_planeswalkers.len());
                    match slot.checked_sub(1) {
                        None => attackers.push((id, *defending_player)),
                        Some(w) => planeswalker_attacks.push((id, defending_planeswalkers[w])),
                    }
                }
                Action::DeclareAttackers { attackers, planeswalker_attacks }
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

#[cfg(test)]
mod tests {
    use super::*;
    use mtg_engine::ids::{ObjectId, PlayerId};

    fn attackers_prompt(eligible: &[u64], walkers: &[u64]) -> CombatPrompt {
        CombatPrompt::ChooseAttackers {
            eligible: eligible.iter().map(|&i| ObjectId(i)).collect(),
            must_attack: vec![],
            defending_player: PlayerId(1),
            defending_planeswalkers: walkers.iter().map(|&i| ObjectId(i)).collect(),
        }
    }

    /// A random seat must be able to send an attacker at a planeswalker.
    ///
    /// `planeswalker_attacks` was hard-coded empty, so across 30 seeded games
    /// with 464 planeswalkers on the battlefield and 303 attack declarations,
    /// a walker was attacked exactly zero times — leaving every CR 508.1a
    /// property invisible to the fuzzer that is supposed to be hunting them
    /// (issue #220).
    #[test]
    fn a_random_seat_attacks_planeswalkers_when_the_defender_has_them() {
        let mut player = RandomPlayer::with_seed("r", 1);
        let prompt = attackers_prompt(&[10, 11, 12, 13], &[20, 21]);

        let mut at_player = 0;
        let mut at_walker = 0;
        for _ in 0..200 {
            let Action::DeclareAttackers { attackers, planeswalker_attacks } =
                player.choose_combat(&prompt)
            else {
                panic!("an attackers prompt is answered with a declaration");
            };
            at_player += attackers.len();
            at_walker += planeswalker_attacks.len();
        }

        assert!(at_walker > 0, "a random seat sends attackers at walkers (got {at_walker})");
        assert!(at_player > 0, "and still attacks the player too (got {at_player})");
        // Uniform over {player, walker 20, walker 21}: roughly a third each.
        // The bound is loose enough not to be a flake, tight enough that
        // "walkers are attacked once in a blue moon" would fail it.
        let total = at_player + at_walker;
        assert!(at_walker * 3 > total, "walkers are a real share of the targets, \
            not a rounding error: {at_walker} of {total}");
    }

    /// Whatever it picks must be a declaration the engine offered: every
    /// walker attacked is one the prompt listed, and no creature attacks twice.
    #[test]
    fn a_random_declaration_only_names_offered_attackers_and_walkers() {
        let mut player = RandomPlayer::with_seed("r", 7);
        let eligible = [10, 11, 12, 13];
        let walkers = [20, 21];
        let prompt = attackers_prompt(&eligible, &walkers);

        for _ in 0..200 {
            let Action::DeclareAttackers { attackers, planeswalker_attacks } =
                player.choose_combat(&prompt)
            else {
                panic!("an attackers prompt is answered with a declaration");
            };
            let mut declared: Vec<ObjectId> = Vec::new();
            for (id, defender) in &attackers {
                assert!(eligible.contains(&id.0), "attacker {id:?} was offered");
                assert_eq!(*defender, PlayerId(1), "the player attacked is the defender");
                declared.push(*id);
            }
            for (id, walker) in &planeswalker_attacks {
                assert!(eligible.contains(&id.0), "attacker {id:?} was offered");
                assert!(walkers.contains(&walker.0), "walker {walker:?} was offered");
                declared.push(*id);
            }
            let mut unique = declared.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), declared.len(),
                "no creature attacks two defenders at once: {declared:?}");
        }
    }

    /// With no walkers to attack, the declaration is exactly what it always
    /// was — every attacker at the player.
    #[test]
    fn with_no_walkers_every_attacker_still_goes_at_the_player() {
        let mut player = RandomPlayer::with_seed("r", 3);
        let prompt = attackers_prompt(&[10, 11, 12], &[]);
        let mut attacked = 0;
        for _ in 0..100 {
            let Action::DeclareAttackers { attackers, planeswalker_attacks } =
                player.choose_combat(&prompt)
            else {
                panic!("an attackers prompt is answered with a declaration");
            };
            assert!(planeswalker_attacks.is_empty(), "no walkers, no walker attacks");
            attacked += attackers.len();
        }
        assert!(attacked > 0, "it still attacks (got {attacked})");
    }
}

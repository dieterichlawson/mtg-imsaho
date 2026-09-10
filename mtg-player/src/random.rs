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

/// How often this seat backs out of a cast-time prompt instead of
/// answering it.
///
/// Every one of the engine's four `CancelCast` un-stash arms — the fixes
/// for #123, #262 and #290 — was unreachable to the fuzzer, because the
/// only seat it plays always answered with a set or a funding response:
/// 547 structured cast prompts in 70 seeded games, zero cancelled. Those
/// arms are exactly where a spell or an activation gets stranded if the
/// un-stash is wrong, which is the structural wrongness
/// `--check-invariants` exists to catch (issue #457).
///
/// Small on purpose: a seat that cancels often stops casting X spells,
/// which would trade this coverage for the coverage that already works.
const CANCEL_CHANCE: f64 = 0.05;

/// How often this seat mulligans a hand it is offered.
///
/// It never did. 140 mulligan decisions over 70 seeded games, 140 keeps —
/// so `MulliganMull`, the whole `BottomAfterMulligan` half of the London
/// mulligan, and every invariant written for that path had never been seen
/// by a fuzz game (issue #456). The old comment called keeping a
/// "deterministic mulligan policy", but a seeded roll is just as
/// deterministic; what the constant bought was a stable opening hand, not
/// reproducibility.
const MULLIGAN_CHANCE: f64 = 0.25;

/// The seat's own cap on mulligans. CR 103.4 has none — #63 is about
/// exactly that — so the cap belongs here, where a pathological seed would
/// otherwise mulligan a game away. Three is enough to reach the bottoming
/// prompt, and to reach it with more than one card to bottom.
const MAX_MULLIGANS: u32 = 3;

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

impl RandomPlayer {
    /// Back out of the cast this prompt belongs to?
    ///
    /// Only ever asked at the three prompt kinds the engine accepts
    /// `CancelCast` for — the two `ChooseXFunding` arms, `ChooseTargetSet`
    /// and `ChooseExileFromGraveyard`. Anywhere else it is an answer of the
    /// wrong shape, which the engine refuses while leaving the question
    /// standing, and a seat that answered that way would spin (#457).
    fn cancels_the_cast(&mut self) -> bool {
        self.rng.gen_bool(CANCEL_CHANCE)
    }
}

impl Player for RandomPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
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
            if self.cancels_the_cast() {
                return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
            }
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

        // Exile-from-graveyard additional cost. The count is rolled across
        // the whole range, not taken at the minimum.
        //
        // For a fixed-count cost — Stitched Drake, Makeshift Mauler, Corpse
        // Lunge, Skaab Goliath, Skaab Ruinator — `min == max` and the roll
        // is the forced answer either way. For `ExileXFromGraveyard` the
        // count IS X, and `min` is zero: taking it meant this seat cast
        // Harvest Pyre 191 times in 20 seeded games, exiled nothing every
        // time, and dealt 0 damage every time, which CR 120.8 makes nothing
        // at all. The one card in the pool with that cost had its whole
        // "this spell does damage" half invisible to the fuzzer (#455).
        //
        // A random subset rather than the first `how_many`, for the reason
        // the target set below gives: in order, the oldest cards in the
        // graveyard are the only ones ever exiled.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
            options, min, max, ..
        }) = legal.resolution_prompt.as_ref()
        {
            use mtg_engine::actions::ResolvedChoice;
            use rand::seq::SliceRandom;
            if self.cancels_the_cast() {
                return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
            }
            let how_many = if max > min { self.rng.gen_range(*min..=*max) } else { *min };
            let chosen: Vec<mtg_engine::ids::ObjectId> =
                options.choose_multiple(&mut self.rng, how_many).copied().collect();
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
            if self.cancels_the_cast() {
                return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
            }
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
        // for this player, a list to index into for no benefit. A random
        // subset of the size asked for: the first `min` in hand order is
        // the same card every time a hand has the same shape, so a
        // bottoming or a discard never reaches past the front of the hand.
        if let Some(prompt) = legal.set_prompt.as_ref() {
            use rand::seq::SliceRandom;
            let chosen: Vec<mtg_engine::ids::ObjectId> =
                prompt.options.choose_multiple(&mut self.rng, prompt.min).copied().collect();
            return prompt.answer(chosen);
        }

        // Mulligan: rolled, and capped by this seat rather than by the
        // rules (CR 103.4 has no cap — issue #63 — so a pathological seed
        // would otherwise mulligan a game away).
        if let Some(keep_idx) = legal_actions.iter().position(|a| matches!(a, Action::MulliganKeep)) {
            let offered_mull = legal_actions.iter().any(|a| matches!(a, Action::MulliganMull));
            if offered_mull
                && view.your_mulligan_count < MAX_MULLIGANS
                && self.rng.gen_bool(MULLIGAN_CHANCE)
            {
                return Action::MulliganMull;
            }
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

// ── The seat that answers with a constant (issues #455, #456, #457) ──────
//
// `CLAUDE.md`: "Never let a non-interactive seat answer with a constant
// where the constant is a legal no-op. Roll it, or the fuzzer covers
// nothing." Three arms of this seat broke it, and each one made a whole
// class of engine code unreachable to the only seat the invariant fuzzer
// plays. These are the guards that were missing.
#[cfg(test)]
mod rolls {
    use super::*;
    use mtg_engine::actions::{ResolvedChoice, SetPrompt, SetPromptKind};
    use mtg_engine::engine::LegalActions;
    use mtg_engine::ids::{ObjectId, PlayerId};
    use mtg_engine::state::ResolutionChoiceKind;
    use mtg_engine::types::{ManaPool, Step};
    use std::collections::HashMap;

    fn view() -> GameView {
        GameView {
            you: PlayerId(0),
            your_hand: vec![],
            your_life: 20,
            your_mana_pool: ManaPool::new(),
            your_library_size: 40,
            your_library_cards: vec![],
            your_mulligan_count: 0,
            opponents: vec![],
            battlefield: vec![],
            graveyards: vec![],
            stack: vec![],
            exile: vec![],
            first_strike_damage_step: false,
            step: Step::PrecombatMain,
            active_player: PlayerId(0),
            priority_player: Some(PlayerId(0)),
            turn_number: 1,
            display_log: vec![],
            full_log: vec![],
            revealed_names: HashMap::new(),
        }
    }

    fn prompted(kind: ResolutionChoiceKind) -> LegalActions {
        LegalActions {
            actions: vec![],
            combat_prompt: None,
            castable_spells: vec![],
            activatable_abilities: vec![],
            context: None,
            resolution_prompt: Some(kind),
            set_prompt: None,
        }
    }

    fn ids(n: u64) -> Vec<ObjectId> {
        (0..n).map(ObjectId).collect()
    }

    fn exile_prompt(min: usize, max: usize) -> LegalActions {
        prompted(ResolutionChoiceKind::ChooseExileFromGraveyard {
            description: "exile".into(),
            options: ids(8),
            min,
            max,
            source_id: ObjectId(99),
        })
    }

    fn funding_prompt(is_ability: bool) -> LegalActions {
        prompted(ResolutionChoiceKind::ChooseXFunding {
            description: "X".into(),
            options: mtg_engine::funding::FundingOptions {
                pool: std::collections::BTreeMap::new(),
                groups: vec![],
                max_x: 0,
                x_discount: 0,
            },
            source_id: ObjectId(99),
            is_ability,
        })
    }

    /// Answer `n` times and report what came back.
    fn answers(legal: &LegalActions, n: usize) -> Vec<Action> {
        let mut p = RandomPlayer::with_seed("r", 7);
        let v = view();
        (0..n).map(|_| p.choose_action(&v, legal)).collect()
    }

    fn cancels(legal: &LegalActions, n: usize) -> usize {
        answers(legal, n).iter()
            .filter(|a| matches!(a, Action::ResolveChoice { choice: ResolvedChoice::CancelCast }))
            .count()
    }

    /// Issue #455: the count exiled for an `ExileXFromGraveyard` cost IS X,
    /// and `min` is zero — so a seat that took the minimum cast Harvest
    /// Pyre 191 times in 20 seeded games, exiled nothing every time, and
    /// dealt zero damage every time. The whole "this spell does damage"
    /// half of the one card in the pool with that cost was invisible.
    #[test]
    fn the_exile_cost_rolls_its_count_across_the_whole_range() {
        let mut seen = std::collections::HashSet::new();
        for a in answers(&exile_prompt(0, 5), 400) {
            if let Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(set) } = a {
                assert!(set.len() <= 5, "never past max: {}", set.len());
                let mut sorted = set.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(sorted.len(), set.len(), "no card exiled twice: {set:?}");
                seen.insert(set.len());
            }
        }
        for n in 0..=5 {
            assert!(seen.contains(&n), "X={n} is reachable; saw {seen:?}");
        }

        // A fixed-count cost — Stitched Drake and the rest — has one legal
        // answer and still gets it.
        for a in answers(&exile_prompt(2, 2), 60) {
            if let Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(set) } = a {
                assert_eq!(set.len(), 2);
            }
        }
    }

    /// And which cards, not only how many: taking them in graveyard order
    /// means the oldest cards are the only ones ever exiled.
    #[test]
    fn the_exile_cost_reaches_every_card_in_the_graveyard() {
        let mut seen = std::collections::HashSet::new();
        for a in answers(&exile_prompt(1, 1), 400) {
            if let Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(set) } = a {
                seen.extend(set);
            }
        }
        assert_eq!(seen.len(), 8, "every card in the graveyard is reachable: {seen:?}");
    }

    /// Issue #457: every one of the engine's four `CancelCast` un-stash arms
    /// — the fixes for #123, #262 and #290 — was unreachable, because this
    /// seat always answered a cast-time prompt with an answer. 547
    /// structured cast prompts in 70 seeded games, zero cancelled.
    #[test]
    fn a_cast_time_prompt_is_sometimes_declined() {
        for (what, legal) in [
            ("an X-cost spell (#123)", funding_prompt(false)),
            ("an X-cost ability (#290)", funding_prompt(true)),
            ("an exile cost (#262)", exile_prompt(0, 3)),
            ("a target set", prompted(ResolutionChoiceKind::ChooseTargetSet {
                description: "targets".into(),
                options: vec![],
                fixed: vec![],
                min: 0,
                max: 2,
                source_id: ObjectId(99),
            })),
        ] {
            let n = cancels(&legal, 400);
            assert!(n > 0, "{what}: the seat never backs out");
            assert!(n < 100, "{what}: and does not do it often enough to stop \
                casting X spells altogether ({n} of 400)");
        }
    }

    /// Cancel is only an answer to those three. Anywhere else the engine
    /// refuses it and leaves the question standing, which would wedge a
    /// random game into a loop — so the seat must never produce it there.
    #[test]
    fn nothing_else_is_ever_declined() {
        let object_set = prompted(ResolutionChoiceKind::ChooseObjectSet {
            description: "objects".into(),
            options: ids(4),
            min: 0,
            max: 2,
            effect: mtg_engine::state::PendingEffect::DealDamage {
                amount: 1,
                source_id: ObjectId(99),
            },
        });
        let piles = prompted(ResolutionChoiceKind::DividePermanentsIntoPiles {
            description: "piles".into(),
            permanents: ids(4),
            target_player: PlayerId(1),
            source_id: ObjectId(99),
        });
        for (what, legal) in [("an object set", object_set), ("a pile split", piles)] {
            assert_eq!(cancels(&legal, 400), 0,
                "{what} does not accept CancelCast; answering it that way \
                 leaves the question standing");
        }
    }

    /// Issue #456: 140 mulligan decisions over 70 seeded games, 140 keeps.
    /// `MulliganMull`, the whole `BottomAfterMulligan` half of the London
    /// mulligan, and every invariant written for that path had never been
    /// seen by a fuzz game.
    #[test]
    fn the_opening_hand_is_sometimes_mulliganed_and_the_seat_stops() {
        let mull_or_keep = LegalActions {
            actions: vec![Action::MulliganKeep, Action::MulliganMull],
            combat_prompt: None,
            castable_spells: vec![],
            activatable_abilities: vec![],
            context: None,
            resolution_prompt: None,
            set_prompt: None,
        };
        let mut p = RandomPlayer::with_seed("r", 7);
        let v = view();
        let (mut kept, mut mulled) = (0, 0);
        for _ in 0..400 {
            match p.choose_action(&v, &mull_or_keep) {
                Action::MulliganKeep => kept += 1,
                Action::MulliganMull => mulled += 1,
                other => panic!("a mulligan prompt is answered with one of the two: {other:?}"),
            }
        }
        assert!(mulled > 0, "the seat mulligans");
        assert!(kept > mulled, "and keeps more often than it does not: {kept}/{mulled}");

        // Capped by the seat, because CR 103.4 caps nothing (#63): at the
        // cap it always keeps, so a pathological seed cannot mulligan a
        // game away.
        let mut at_cap = v.clone();
        at_cap.your_mulligan_count = MAX_MULLIGANS;
        let mut p = RandomPlayer::with_seed("r", 7);
        for _ in 0..200 {
            assert!(matches!(p.choose_action(&at_cap, &mull_or_keep), Action::MulliganKeep),
                "at {MAX_MULLIGANS} mulligans the seat keeps whatever it is dealt");
        }
    }

    /// The bottoming and cleanup-discard answer reaches past the front of
    /// the hand: the first `min` in hand order is the same card every time.
    #[test]
    fn a_card_set_is_marked_across_the_whole_hand() {
        let legal = LegalActions {
            actions: vec![],
            combat_prompt: None,
            castable_spells: vec![],
            activatable_abilities: vec![],
            context: None,
            resolution_prompt: None,
            set_prompt: Some(SetPrompt {
                kind: SetPromptKind::BottomAfterMulligan,
                player: PlayerId(0),
                options: ids(7),
                min: 1,
                max: 1,
            }),
        };
        let mut seen = std::collections::HashSet::new();
        for a in answers(&legal, 400) {
            if let Action::BottomCards { cards } = a {
                assert_eq!(cards.len(), 1, "exactly the count asked for");
                seen.extend(cards);
            }
        }
        assert_eq!(seen.len(), 7, "every card in hand can be bottomed: {seen:?}");
    }
}

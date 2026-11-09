//! A decision is never asked for with nothing on offer.
//!
//! `LegalActions::offers_nothing()` — no actions, no combat prompt, no
//! resolution prompt, no set prompt — is the shape a player implementation
//! cannot answer. `run_game_loop` has three callback sites; two are
//! preceded by an `offers_nothing()` guard that `continue`s and the third
//! synthesises a two-action menu, so no callback can ever be handed one.
//!
//! That made `mtg-runner`'s own check — "no legal actions and no prompt:
//! the game is stuck", inside the per-decision `--check-invariants` block —
//! dead code: the single line in the program that would report a stuck game
//! was in the one place that cannot see it, while the branch that runs
//! instead was the loop's only unbounded, uncounted, unlogged path (issue
//! #498). The reporting now lives in the loop, which counts that branch and
//! ends a game that cannot move as a draw (CR 104.4b).
//!
//! What is worth testing is the reason the branch is unreachable, which is
//! not luck: every way a player can be on the hook for a decision produces
//! something for them to answer. That is a property with a live failure
//! mode — `offers_nothing()`'s own doc comment records a fourth prompt kind
//! being added and three of four copies of this question not learning about
//! it — and the sweep below is exhaustive by construction: a new
//! `AwaitingAction` variant fails to compile here until someone says what
//! it offers.

mod common;
use common::*;
use mtg_engine::engine::legal_actions;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
use mtg_engine::types::*;

/// Every `AwaitingAction` a player can be waiting on offers them something.
#[test]
fn every_pending_decision_offers_the_player_something_to_answer() {
    let reg = registry();

    // Exhaustive by construction: `match` on a value of the enum, so
    // adding a variant is a compile error here until it is listed.
    fn all_variants(bear: ObjectId) -> Vec<(&'static str, AwaitingAction)> {
        let sample = AwaitingAction::DeclareAttackers;
        #[allow(unreachable_patterns)]
        match sample {
            AwaitingAction::DeclareAttackers
            | AwaitingAction::DeclareBlockers { .. }
            | AwaitingAction::DiscardToHandSize { .. }
            | AwaitingAction::ResolutionChoice { .. }
            | AwaitingAction::MulliganDecision { .. }
            | AwaitingAction::BottomAfterMulligan { .. } => {}
        }
        vec![
            ("DeclareAttackers", AwaitingAction::DeclareAttackers),
            ("DeclareBlockers", AwaitingAction::DeclareBlockers { defending_player: P1 }),
            // Zero to discard is the boundary the mulligan loop's own
            // safety comment names ("zero cards to bottom").
            ("DiscardToHandSize(0)",
                AwaitingAction::DiscardToHandSize { player: P0, discard_count: 0 }),
            ("DiscardToHandSize(2)",
                AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 }),
            ("ResolutionChoice", AwaitingAction::ResolutionChoice {
                player: P0,
                source: bear,
                choice: ResolutionChoiceKind::YesNo {
                    description: "?".into(), source_card: bear },
            }),
            ("MulliganDecision", AwaitingAction::MulliganDecision { player: P0 }),
            ("BottomAfterMulligan(0)",
                AwaitingAction::BottomAfterMulligan { player: P0, count: 0 }),
            ("BottomAfterMulligan(1)",
                AwaitingAction::BottomAfterMulligan { player: P0, count: 1 }),
        ]
    }

    for step in [Step::PrecombatMain, Step::DeclareAttackers, Step::DeclareBlockers,
                 Step::Cleanup] {
        let mut state = game_at_step(step, P0);
        let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        // A player waiting on a decision does not also hold priority.
        state.priority_player = None;
        for (name, awaiting) in all_variants(bear) {
            state.awaiting_action = Some(awaiting);
            let legal = legal_actions(&state, &reg);
            assert!(
                !legal.offers_nothing(),
                "{name} at {step:?} leaves the player nothing to answer: no actions, no \
                 combat prompt, no resolution prompt, no set prompt. A player \
                 implementation cannot answer that and the loop cannot ask it — it \
                 would take the advance-and-retry branch instead, forever."
            );
        }
    }
}

/// And a player who merely has priority always has at least the two
/// actions CR 117.1 guarantees them.
#[test]
fn a_player_with_priority_can_always_pass_or_concede() {
    use mtg_engine::actions::Action;
    let reg = registry();
    for step in [Step::Upkeep, Step::PrecombatMain, Step::DeclareBlockers,
                 Step::EndStep, Step::Cleanup] {
        let mut state = game_at_step(step, P0);
        state.awaiting_action = None;
        let legal = legal_actions(&state, &reg);
        assert!(!legal.offers_nothing(), "priority at {step:?} offers nothing");
        assert!(legal.actions.iter().any(|a| matches!(a, Action::PassPriority)),
            "pass at {step:?}");
        assert!(legal.actions.iter().any(|a| matches!(a, Action::Concede)),
            "concede at {step:?}");
    }
}

/// The two questions callers ask of an offer, as the game loops and the
/// seats depend on them: `offers_nothing` is what the loops skip a callback
/// on, and `is_observed_pass` is what a watching seat answers exactly as the
/// loop would have (#956). Each holds for its own shape and no other.
#[test]
fn an_offer_says_whether_it_is_empty_or_an_observed_pass() {
    use mtg_engine::engine::LegalActions;
    let pass = LegalActions::observed_pass();
    assert!(pass.is_observed_pass());
    assert!(!pass.offers_nothing(), "a pass and a concession are two things to answer");

    let mut empty = LegalActions::observed_pass();
    empty.actions.clear();
    assert!(empty.offers_nothing(), "no actions and no prompt is nothing to answer");
    assert!(!empty.is_observed_pass(), "and not a pass");

    let mut explained = LegalActions::observed_pass();
    explained.context = Some("Declare blockers".into());
    assert!(!explained.is_observed_pass(),
        "a pass the engine explains is a decision the seat sees, not the loop's own pass");

    let mut prompted = LegalActions::observed_pass();
    prompted.resolution_prompt = Some(ResolutionChoiceKind::ChooseDamageAssignmentOrder {
        description: String::new(), attacker: ObjectId(1), remaining: vec![], options: vec![],
    });
    assert!(!prompted.is_observed_pass());
}

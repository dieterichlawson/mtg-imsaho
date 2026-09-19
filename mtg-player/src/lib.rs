pub mod random;
pub mod cli;
pub mod gui;
pub mod llm;
pub mod scripted;
pub mod game_log;
pub mod watchdog;

use mtg_engine::view::GameView;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::engine::LegalActions;

/// The answer to a combat prompt that has only one, or `None` when the
/// player has a choice to make.
///
/// The engine is right to ask: CR 508.1 makes declaring attackers a
/// turn-based action the active player performs every combat, declaring
/// none included, and CR 509.1 does the same for blocks. But a question
/// with exactly one legal answer is not a question to put to a person, and
/// with nothing eligible the empty declaration is the only answer there is.
///
/// This rule was written four times — `cli.rs` guarded both halves, `llm.rs`
/// guarded both halves, and the random seat produced the empty declaration
/// by rolling over an empty list — and the page, which is the fourth
/// surface and the newest, had neither half. So the browser drew
/// "CLICK CREATURES TO ATTACK WITH, THEN CONFIRM" over a board with nothing
/// to click, on most of the first four turns of every game and after every
/// wipe: 8 of 11 declare-blockers prompts in three measured games were
/// screens with nothing on them (issue #517).
///
/// One copy, applied at each seat's combat entry point, so the next seat
/// gets it by asking rather than by remembering.
#[must_use]
pub fn forced_combat_answer(prompt: &CombatPrompt) -> Option<Action> {
    match prompt {
        // `must_attack` is a subset of `eligible`, so an empty `eligible`
        // already implies no requirement is outstanding; it is named here
        // because "nothing may attack" and "something must" are the two
        // things that decide whether there is a choice.
        CombatPrompt::ChooseAttackers { eligible, must_attack, .. }
            if eligible.is_empty() && must_attack.is_empty() =>
            Some(Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] }),
        // No blocker, or nothing attacking to block. The engine raises this
        // prompt only with an attacker present, so the second half is
        // defence rather than a live case.
        CombatPrompt::ChooseBlockers { eligible_blockers, attackers, .. }
            if eligible_blockers.is_empty() || attackers.is_empty() =>
            Some(Action::DeclareBlockers { assignments: vec![] }),
        _ => None,
    }
}

/// The Player trait: given a view of the game and legal actions, pick one.
pub trait Player {
    fn name(&self) -> &str;

    /// Choose an action given the full legal actions structure.
    /// For most players, only `legal.actions` matters. The CLI uses
    /// `legal.castable_spells` for interactive target selection.
    fn choose_action(&mut self, view: &GameView, legal: &LegalActions) -> Action;
}

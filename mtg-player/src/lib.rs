pub mod random;
pub mod cli;
pub mod gui;
pub mod llm;
pub mod scripted;
pub mod game_log;
pub mod watchdog;

use mtg_engine::view::GameView;
use mtg_engine::actions::{Action, CastableSpell, CombatPrompt};
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
/// What a cast row says about the cost it will pay, or `None` when the row
/// pays the printed cost and there is nothing to add.
///
/// Three surfaces render a cast row and each had its own copy of this match,
/// all three gated on `!is_flashback` — the verb "Flashback" says what KIND
/// of cost it is, and calling a printed flashback cost an "alternative cost"
/// read as a discount that was not there (issue #300). So no surface ever
/// said the amount. CR 702.33 lets one card in the graveyard carry several
/// instances of flashback at once — Past in Flames grants one equal to the
/// card's mana cost, alongside the printed one — and CR 601.2b makes which
/// to pay the caster's choice. The only other thing a flashback row carried
/// was its tap plan, which is empty once the mana is already floating, so
/// two different costs rendered one byte-identical string and each surface's
/// label dedupe dropped one of them (issue #611).
///
/// A row states the cost it charges. One copy, so the next surface gets it
/// by asking rather than by remembering.
#[must_use]
pub fn cast_cost_note(cs: &CastableSpell) -> Option<String> {
    let alt = cs.alternative_cost.as_ref()?;
    // An empty `ManaCost` Displays as nothing at all, so a free cost is
    // named rather than trailing off after the word.
    let amount = if alt.symbols.is_empty() { "{0}".to_string() } else { alt.to_string() };
    Some(if cs.is_flashback {
        format!("flashback cost {amount}")
    } else if alt.symbols.is_empty() {
        "without paying its mana cost".to_string()
    } else {
        format!("alternative cost {amount}")
    })
}

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

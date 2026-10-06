pub mod random;

/// `eprintln!`, except that a write that fails is dropped instead of
/// panicking.
///
/// The runtime ignores SIGPIPE, so `eprintln!` to a pipe whose reader has
/// gone — stderr through `head`, or a log shipper that died — panics. In a
/// seat's retry line that turned a call failure the seat would have ridden
/// out into a panic: `mtg-runner` exited 101, and a draft tournament exited
/// 1 with no reason written anywhere (#652, #685). Every runtime line on
/// stderr in the game backends and both runners goes through this.
#[macro_export]
macro_rules! stderr_line {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

/// [`stderr_line!`] without the newline: the `eprint!` counterpart.
#[macro_export]
macro_rules! stderr_text {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = write!(std::io::stderr(), $($arg)*);
    }};
}

pub mod cli;
pub mod gui;
pub mod llm;
pub mod scripted;
pub mod game_log;
pub mod watchdog;

use mtg_engine::view::GameView;
use mtg_engine::actions::{Action, CastableSpell, CombatPrompt};
use mtg_engine::engine::LegalActions;
use mtg_engine::ids::{CardId, ObjectId};
use mtg_engine::types::ManaCost;

#[cfg(test)]
mod surface_parity;

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

/// The key the engine files a cast offer under: the object, and the cost
/// it pays spelled the way `invariants/legal.rs` (`distinct_offers`,
/// `collapsed_views`) spells it.
pub type CastOfferKey = (ObjectId, String);

/// The key the engine files an activation offer under: the permanent,
/// which of its abilities, and — for an ability an Aura granted — whose
/// ability it is.
pub type AbilityOfferKey = (ObjectId, usize, Option<CardId>);

/// The offer key of a way to cast, from either half of `LegalActions`: a
/// `CastSpell` action's fields, or a `CastableSpell`'s.
///
/// Two surfaces collapse `legal.actions` into one row per way to cast, and
/// each carried its own copy of this key. The copies drifted from the
/// engine's at different times — the LLM seat's until #589, the CLI's
/// until #610, each keyed on `alternative_cost.is_some()`, one bit for the
/// two flashback costs CR 702.33 lets one card carry — and each drift
/// dropped a legal option from one surface while the other still offered
/// it. Nothing failed: both of the engine's lists agree with each other, so
/// `--check-invariants` cannot see a row a surface never rendered (the
/// 2026-09-29 playtest's method notes). One copy, so the next surface gets
/// it by asking, and `surface_parity.rs` holds the surfaces to it.
#[must_use]
pub fn cast_offer_key(object_id: ObjectId, alternative_cost: Option<&ManaCost>) -> CastOfferKey {
    (object_id, format!("{alternative_cost:?}"))
}

/// The offer key of an activation, from either half of `LegalActions`: an
/// `ActivateAbility` action's fields, or an `ActivatableAbility`'s.
///
/// The LLM seat keyed on the pair without `source_card_id` and so dropped
/// every Aura- or Equipment-granted ability whose index collided with one
/// the host already had natively (#589). See `cast_offer_key` for why one
/// copy.
#[must_use]
pub fn ability_offer_key(
    object_id: ObjectId, ability_index: usize, source_card_id: Option<CardId>,
) -> AbilityOfferKey {
    (object_id, ability_index, source_card_id)
}

/// The Player trait: given a view of the game and legal actions, pick one.
pub trait Player {
    fn name(&self) -> &str;

    /// Choose an action given the full legal actions structure.
    /// For most players, only `legal.actions` matters. The CLI uses
    /// `legal.castable_spells` for interactive target selection.
    fn choose_action(&mut self, view: &GameView, legal: &LegalActions) -> Action;

    /// Why this seat has stopped answering for good, if it has — an LLM seat
    /// whose backend spent its whole retry budget without an answer. A
    /// runner forfeits such a seat rather than playing on with fallbacks
    /// (#587). Every other seat always answers.
    fn gave_up(&self) -> Option<String> {
        None
    }
}

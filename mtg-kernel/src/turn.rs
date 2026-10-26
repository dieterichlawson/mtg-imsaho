//! The turn: which step follows which, and what all players passing
//! priority in succession does, proved against CR 500-514 and 117 for every
//! step and every combination of the facts that change the sequence.
//!
//! A turn is CR 500.1's twelve steps in order. Exactly four facts change
//! what follows a step, each tied to one step:
//!
//! - the upkeep of the first turn of a two-player game is followed by the
//!   precombat main phase: the player on the play skips their first draw
//!   step, the whole step (CR 103.7a);
//! - a declare attackers step in which no creature attacked is followed by
//!   the end of combat step (CR 508.8);
//! - a combat damage step in which first strikers dealt their damage is
//!   followed by a second combat damage step (CR 510.4);
//! - a cleanup step in which a player received priority is followed by
//!   another cleanup step (CR 514.3a).
//!
//! [`next_step`] is what `engine::advance_step` follows, and [`after_pass`]
//! is what the game loop does when a player passes priority.

use vstd::prelude::*;

verus! {

pub const UNTAP: u8 = 0;
pub const UPKEEP: u8 = 1;
pub const DRAW: u8 = 2;
pub const PRECOMBAT_MAIN: u8 = 3;
pub const BEGIN_COMBAT: u8 = 4;
pub const DECLARE_ATTACKERS: u8 = 5;
pub const DECLARE_BLOCKERS: u8 = 6;
pub const COMBAT_DAMAGE: u8 = 7;
pub const END_COMBAT: u8 = 8;
pub const POSTCOMBAT_MAIN: u8 = 9;
pub const END_STEP: u8 = 10;
pub const CLEANUP: u8 = 11;

/// What, besides the step itself, decides the step after it.
#[derive(Clone, Copy)]
pub struct StepFacts {
    /// First and double strikers dealt their damage in this combat damage
    /// step, so the regular one follows (CR 510.4).
    pub second_damage_step_due: bool,
    /// A player received priority in this cleanup step, so another
    /// cleanup step follows (CR 514.3a).
    pub cleanup_repeats: bool,
    /// At least one creature was declared as an attacker (CR 508.8).
    pub attackers_declared: bool,
    /// This is the first turn of a two-player game, whose player skips its
    /// draw step (CR 103.7a).
    pub skips_first_draw: bool,
}

/// A step is skipped from `from` when the rules say so: the first turn's
/// draw step, and the declare blockers and combat damage steps of a combat
/// with no attackers. Nothing else is ever skipped; in particular, both
/// main phases happen every turn (CR 500.1).
pub open spec fn skipped(s: u8, from: u8, f: StepFacts) -> bool {
    ||| (s == DRAW && from == UPKEEP && f.skips_first_draw)
    ||| ((s == DECLARE_BLOCKERS || s == COMBAT_DAMAGE) && from == DECLARE_ATTACKERS && !f.attackers_declared)
}

/// A step is followed by itself when the rules repeat it.
pub open spec fn repeats(s: u8, f: StepFacts) -> bool {
    ||| (s == COMBAT_DAMAGE && f.second_damage_step_due)
    ||| (s == CLEANUP && f.cleanup_repeats)
}

/// The step after `step` in the same turn, or `None` when the turn ends.
///
/// - The turn ends exactly after a cleanup step that does not repeat.
/// - A step is followed by itself exactly when the rules repeat it.
/// - Otherwise the next step is later in the turn, and every step between
///   the two is one the rules skip from here; and every skip the rules
///   call for is taken.
///
/// These determine the next step completely, and since every step but a
/// repeated one moves later, a turn with finitely many repeats ends.
pub fn next_step(step: u8, f: StepFacts) -> (next: Option<u8>)
    requires step <= CLEANUP,
    ensures
        next.is_none() <==> (step == CLEANUP && !f.cleanup_repeats),
        next matches Some(n) ==> n <= CLEANUP,
        next == Some(step) <==> repeats(step, f),
        next matches Some(n) ==> (n == step || n > step),
        forall|s: u8| #[trigger] skipped(s, step, f) ==> !(next matches Some(n) && n <= s),
        next matches Some(n) ==> forall|s: u8| step < s < n ==> #[trigger] skipped(s, step, f),
{
    if step == COMBAT_DAMAGE && f.second_damage_step_due {
        Some(COMBAT_DAMAGE)
    } else if step == CLEANUP {
        if f.cleanup_repeats { Some(CLEANUP) } else { None }
    } else if step == DECLARE_ATTACKERS && !f.attackers_declared {
        Some(END_COMBAT)
    } else if step == UPKEEP && f.skips_first_draw {
        Some(PRECOMBAT_MAIN)
    } else {
        Some(step + 1)
    }
}

/// What happens when a player passes priority (CR 117.3d, 117.4).
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AfterPass {
    /// The next player in turn order receives priority.
    NextPlayer(u32),
    /// All players passed in succession with something on the stack: the
    /// top object resolves (CR 117.4, 608.1).
    ResolveTop,
    /// All players passed in succession with the stack empty: the step or
    /// phase ends (CR 117.4, 500.2).
    EndStep,
}

/// After a pass that makes `passes` passes in succession, by `current` in a
/// game of `players` players with `stack_len` objects on the stack.
///
/// - The step ends exactly when every player has passed in succession with
///   the stack empty, and the top of the stack resolves exactly when every
///   player has passed in succession with it non-empty.
/// - Otherwise priority goes to the next player in turn order, wrapping.
///
/// So a round in which no player acts ends after exactly `players` passes,
/// wherever it started.
pub fn after_pass(passes: u32, players: u32, stack_len: usize, current: u32) -> (a: AfterPass)
    requires players >= 1, current < players,
    ensures
        a == AfterPass::EndStep <==> (passes >= players && stack_len == 0),
        a == AfterPass::ResolveTop <==> (passes >= players && stack_len > 0),
        passes < players ==> a == AfterPass::NextPlayer(if current + 1 == players { 0 } else { (current + 1) as u32 }),
        a matches AfterPass::NextPlayer(p) ==> p < players,
{
    if passes >= players {
        if stack_len == 0 { AfterPass::EndStep } else { AfterPass::ResolveTop }
    } else if current + 1 == players {
        AfterPass::NextPlayer(0)
    } else {
        AfterPass::NextPlayer(current + 1)
    }
}

} // verus!

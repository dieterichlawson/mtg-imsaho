//! The state-based actions that are arithmetic, proved against CR 704.5f-h
//! and 704.5q for every toughness, every amount of damage and every number
//! of counters.
//!
//! - A creature with toughness 0 or less is put into its owner's graveyard
//!   (704.5f). That is not destruction: indestructible and regeneration do
//!   not stop it.
//! - Otherwise, a creature with damage marked on it at least its toughness,
//!   or dealt any damage by a source with deathtouch since the last time
//!   state-based actions were checked, is destroyed (704.5g, 704.5h).
//! - A permanent with both +1/+1 and -1/-1 counters on it has N of each
//!   removed, N being the smaller number (704.5q).

use vstd::prelude::*;

verus! {

/// What state-based actions do to a creature.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Fate {
    /// Nothing.
    Stays,
    /// Put into its owner's graveyard for having toughness 0 or less
    /// (704.5f).
    ZeroToughness,
    /// Destroyed, for lethal damage or deathtouch damage (704.5g-h).
    Destroyed,
}

/// The fate of a creature with toughness `toughness`, `damage` damage
/// marked on it, and `deathtouched` when a source with deathtouch dealt it
/// damage since state-based actions were last checked.
pub fn creature_fate(toughness: i32, damage: u32, deathtouched: bool) -> (f: Fate)
    ensures
        f == Fate::ZeroToughness <==> toughness <= 0,
        f == Fate::Destroyed <==> (toughness > 0 && (damage >= toughness || (deathtouched && damage > 0))),
        f == Fate::Stays <==> (toughness > 0 && damage < toughness && !(deathtouched && damage > 0)),
{
    if toughness <= 0 {
        Fate::ZeroToughness
    } else if i64::from(damage) >= i64::from(toughness) || (deathtouched && damage > 0) {
        Fate::Destroyed
    } else {
        Fate::Stays
    }
}

/// The +1/+1 and -1/-1 counters left on a permanent with `plus` and `minus`
/// of them once 704.5q has removed the pairs.
///
/// - The same number of each is removed, and it is the smaller number, so
///   at most one kind is left.
/// - So the counters' net effect on power and toughness is unchanged.
pub fn annihilate(plus: u32, minus: u32) -> (left: (u32, u32))
    ensures
        left.0 <= plus && left.1 <= minus,
        plus - left.0 == minus - left.1,
        left.0 == 0 || left.1 == 0,
        plus - minus == left.0 - left.1,
{
    let n = if plus < minus { plus } else { minus };
    (plus - n, minus - n)
}

} // verus!

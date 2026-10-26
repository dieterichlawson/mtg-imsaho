//! The parts of the rules that are arithmetic over a handful of numbers,
//! proved against the Comprehensive Rules for every input with [Verus].
//!
//! Each function here is ordinary Rust inside a `verus!` block, carrying a
//! contract (`requires`, `ensures`) that says what the rules ask of it in
//! terms of specification functions. `cargo build` erases the contracts and
//! compiles the rest as it would any crate, so `mtg-engine` runs exactly the
//! code that was proved. `scripts/prove.sh` checks every contract with the
//! Z3 SMT solver; unlike a model checker, Verus reasons about integers
//! rather than bits, so a proof here holds for every amount, not for every
//! amount up to a bound.
//!
//! What a reader has to review is the specification functions (`pub open
//! spec fn`): they are the rules written formally, and a proof is only as
//! good as they are. The proof steps between them (`proof` blocks, loop
//! invariants, `assert`s) need no review, because Verus rejects any that do
//! not follow. `docs/plans/formal-verification.md` is the plan this belongs
//! to.
//!
//! [Verus]: https://github.com/verus-lang/verus

pub mod combat;
pub mod funding;
pub mod mana;
pub mod turn;

// `verus!` erases the directive below to an empty block under rustc.
#[allow(unused_braces)]
mod platform {
use vstd::prelude::*;

verus! {

// Every platform the engine is built for has 64-bit pointers, and the
// funding proofs lean on `usize` holding any `u32` plus one.
global size_of usize == 8;

} // verus!
}

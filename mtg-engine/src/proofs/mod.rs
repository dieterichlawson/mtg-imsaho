//! Machine-checked proofs about the engine, run with [Kani].
//!
//! Nothing here is compiled by `cargo build` or `cargo test`: the module is
//! `#[cfg(kani)]`, and `scripts/prove.sh` runs it. Each `#[kani::proof]` is
//! a theorem over *every* input inside its stated bounds, not a sample of
//! them: a passing proof means no input in that space breaks the property,
//! and a failing one comes back with the input that does.
//!
//! Each property is stated against a specification written from the
//! Comprehensive Rules, in the proof's own file, and never against the
//! function under proof, so a proof cannot pass by restating the
//! implementation. `docs/plans/formal-verification.md` is the plan these
//! belong to, and lists what each proof covers and what it does not.
//!
//! [Kani]: https://model-checking.github.io/kani/

mod mana_payment;

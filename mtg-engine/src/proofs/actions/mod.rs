//! The engine's actions, proved against the Comprehensive Rules: what a
//! card proof's recorder stands in for (`proofs::cards` says how the two
//! halves fit). Each action is proved with the actions *it* asks for
//! recorded in turn, so each proof is of one function's own logic.

mod choosing_targets;
mod countering;
mod damage;
mod destruction;
mod effects;
mod fighting;
mod library;
mod moving;
mod pending_effects;
mod predicates;
mod random;
mod scopes;
mod simple_actions;
mod targeting;
mod tokens;
mod transforming;

/// An action proof: the stubs and bounds of a card proof, so that one
/// action's proof uses the next action's recorder.
macro_rules! action_proof {
    ($(#[$meta:meta])* fn $name:ident() $body:block) => {
        $(#[$meta])*
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, $crate::proofs::cards::no_format)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, $crate::proofs::cards::no_growing)]
        #[kani::stub($crate::damage::deal_damage, $crate::proofs::cards::record_damage)]
        #[kani::stub($crate::destruction::try_destroy, $crate::proofs::cards::record_destroy)]
        fn $name() $body
    };
}
pub(crate) use action_proof;

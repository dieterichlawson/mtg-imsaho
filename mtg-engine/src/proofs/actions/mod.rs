//! The engine's actions, proved against the Comprehensive Rules: what a
//! card proof's recorder stands in for (`proofs::cards` says how the two
//! halves fit). Each action is proved with the actions *it* asks for
//! recorded in turn, so each proof is of one function's own logic.

mod activating;
mod answering;
mod attacking;
mod blocking;
mod card_state;
mod casting;
mod casting_costs;
mod choosing_targets;
mod combat_answers;
mod combat_damage;
mod declaring_blocks;
mod countering;
mod damage;
mod delayed;
mod destruction;
mod effects;
mod filters;
mod fighting;
mod library;
mod moving;
mod mulligan;
mod offering;
mod paying;
mod priority;
mod pending_effects;
mod power_and_toughness;
mod predicates;
mod random;
mod resolving;
mod scopes;
mod searching;
mod simple_actions;
mod state_based;
mod targeting;
mod tokens;
mod transforming;
mod trigger_order;
mod trigger_resolving;
mod trigger_targets;
mod triggering;
mod turn_based;
mod until_end_of_turn;

/// An action proof: the stubs and bounds of a card proof, so that one
/// action's proof uses the next action's recorder.
macro_rules! action_proof {
    ($(#[$meta:meta])* fn $name:ident() $body:block) => {
        $(#[$meta])*
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, $crate::proofs::cards::no_format)]
        #[kani::stub($crate::state::GameState::obj_name, $crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, $crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, $crate::proofs::cards::no_freeing)]
        #[kani::stub($crate::damage::deal_damage, $crate::proofs::cards::record_damage)]
        #[kani::stub($crate::destruction::try_destroy, $crate::proofs::cards::record_destroy)]
        fn $name() $body
    };
}
pub(crate) use action_proof;

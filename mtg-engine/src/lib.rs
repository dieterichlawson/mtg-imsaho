// A card proof stands in for over forty engine actions, each a
// `kani::stub` attribute, and each attribute is expanded inside the last.
#![cfg_attr(kani, recursion_limit = "4096")]
// The proofs' stand-in for growing an allocation names the allocator's
// types (`proofs::cards::no_growing`).
#![cfg_attr(kani, feature(allocator_api))]

#[cfg(kani)]
extern crate alloc;

pub mod ids;
pub mod types;
pub mod state;
pub mod events;
pub mod actions;
pub mod mana;
pub mod funding;
pub mod stack;
pub mod combat;
pub mod damage;
pub mod sba;
pub mod engine;
pub mod view;
pub mod cards;
pub mod triggers;
pub mod destruction;
pub mod invariants;
pub mod replacement;
pub mod attachment;
#[cfg(kani)]
pub mod kind_map;
pub mod vec_map;
#[cfg(kani)]
mod proofs;

//! The actions that leave something to chance, which a card proof's
//! `record_random` and `record_shuffle` stand in for: `choose_at_random`,
//! `GameState::shuffle` and `helpers::shuffle_library` (CR 701.20a), all of
//! them by way of `random_below`.
//!
//! What "at random" promises a player is that any of the candidates may be
//! the one, and that the choice is among them: never one twice, never one
//! that was not offered, and as many as asked for when there are that many.
//! The proofs here hold every action to that for every answer the random
//! numbers can give, with `random_below` standing in as any number below
//! its bound (its own proof shows it is one); that each answer is equally
//! likely is the random numbers' own business, and not proved.

use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::new_game;
use crate::state::GameState;

/// Stands in for `GameState::random_below`: any number below `bound`.
#[allow(dead_code)] // named only by `kani::stub`
fn any_below(_: &mut GameState, bound: u64) -> u64 {
    assert!(bound > 0, "random_below needs a positive bound");
    let n: u64 = kani::any();
    kani::assume(n < bound);
    n
}

/// Up to four distinct ids: how many, and the ids.
fn some_ids() -> ([ObjectId; 4], usize) {
    let len: usize = kani::any();
    kani::assume(len <= 4);
    (core::array::from_fn(|i| ObjectId(10 + i as u64)), len)
}

/// Whether `items` holds each of `of` exactly once, and nothing else.
fn is_a_reordering(items: &[ObjectId], of: &[ObjectId]) -> bool {
    items.len() == of.len() && of.iter().all(|x| items.iter().filter(|y| *y == x).count() == 1)
}

#[kani::proof]
#[kani::unwind(3)]
/// `random_below(bound)` is below `bound`, for every state of the game's
/// random numbers and every bound but 0.
fn a_random_number_is_below_its_bound() {
    let mut state = new_game();
    state.rng_state = kani::any();
    let bound: u64 = kani::any();
    kani::assume(bound > 0);
    assert!(state.random_below(bound) < bound);
    std::mem::forget(state);
}

#[kani::proof]
#[kani::unwind(6)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::random_below, any_below)]
/// A shuffle (CR 701.20a) puts the same items in an order: each once, none
/// lost, none added; and any of them may come first. For up to four items.
fn a_shuffle_reorders_and_keeps_every_item() {
    let mut state = new_game();
    let (ids, len) = some_ids();
    let mut items = ids;
    state.shuffle(&mut items[..len]);
    assert!(is_a_reordering(&items[..len], &ids[..len]));
    kani::cover!(len == 4 && items[0] == ids[3], "the last first");
    std::mem::forget(state);
}

#[kani::proof]
#[kani::unwind(6)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::random_below, any_below)]
/// "Choose N at random": N of the candidates, or all of them if there are
/// fewer, none twice and none not offered; and any of them may be among
/// those chosen. For up to four candidates and N up to four.
fn choosing_at_random_chooses_that_many_of_the_candidates() {
    let mut state = new_game();
    let (ids, len) = some_ids();
    let n: usize = kani::any();
    kani::assume(n <= 4);

    let chosen = state.choose_at_random(&ids[..len], n);

    assert!(chosen.len() == n.min(len));
    assert!(chosen.iter().all(|c| ids[..len].contains(c)));
    assert!(chosen.iter().enumerate().all(|(i, c)| !chosen[..i].contains(c)));
    kani::cover!(len == 3 && n == 1 && chosen[0] == ids[2], "the last chosen of three");
    std::mem::forget((state, chosen));
}

#[kani::proof]
#[kani::unwind(6)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::random_below, any_below)]
/// "Shuffle your library" (CR 701.20a): that player's library holds the
/// same cards in an order, the other's is untouched, and the shuffle is
/// announced, once. For either player and libraries of up to four cards.
fn shuffling_a_library_reorders_it_and_announces_it() {
    let mut state = new_game();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let (ids, len) = some_ids();
    let other = PlayerId(1 - who.0);
    // A branch on the player, not an index by it: `any_object` says why.
    for id in &ids[..len] {
        if who == PlayerId(0) {
            state.players[0].library_order.push(*id);
        } else {
            state.players[1].library_order.push(*id);
        }
    }
    let theirs = state.get_player(other).library_order.clone();

    crate::cards::helpers::shuffle_library(&mut state, who);

    assert!(is_a_reordering(&state.get_player(who).library_order, &ids[..len]));
    assert!(state.get_player(other).library_order == theirs);
    assert!(matches!(state.events.as_slice(), [GameEvent::LibraryShuffled { player }] if *player == who));
    std::mem::forget((state, theirs));
}

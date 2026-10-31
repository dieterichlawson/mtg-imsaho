//! The lists a card keeps in its own state while its resolution waits on a
//! player's answer, which a card proof's `record_store_ids`,
//! `record_load_ids` and `record_clear_ids` stand in for:
//! `cards::store_ids`, `cards::load_ids` and `cards::clear_ids`, over the
//! state's map (a `VecMap` in a proof, as in the card proofs).
//!
//! What the stand-ins promise a card is that a list reads back as it was
//! last stored under its prefix, reads back as none once cleared or never
//! stored, and leaves every other entry of the state as it was: a list
//! stored over a longer one leaves none of the longer one's ids behind,
//! and two prefixes keep two lists. The proofs here hold the helpers to
//! that over a state that already holds an entry of another card's
//! (`last_attached_to`), for each length of list a card proof keeps up to
//! three, each pair of lengths a harness of its own, and for the two
//! prefixes Liliana of the Veil keeps side by side.

use crate::cards::{clear_ids, load_ids, store_ids};
use crate::ids::ObjectId;
use crate::proofs::actions::action_proof;
use crate::state::CardState;

/// A state holding one entry that is no list's, with room for every entry
/// a proof here adds (a list that grows is one a proof follows into two
/// buffers, and the proofs' allocator stands in for growing as a failure).
fn state_with_another_entry() -> CardState {
    let mut state = CardState::new();
    state.reserve(8);
    state.insert("last_attached_to".into(), ObjectId(99));
    state
}

/// The entry [`state_with_another_entry`] made, as it was, and nothing
/// else but `n` more entries.
fn keeps_the_other_entry(state: &CardState, n: usize) -> bool {
    state.get("last_attached_to") == Some(&ObjectId(99)) && state.len() == n + 1
}

/// A list of `BEFORE` ids stored, then one of `AFTER` in its place, then
/// cleared: each read back as stored, the first gone once the second is
/// stored (its length and its ids, whichever is longer), and nothing of
/// either once cleared, the other entry as it was throughout.
fn storing_over_then_clearing<const BEFORE: usize, const AFTER: usize>() {
    let mut state = state_with_another_entry();
    assert!(load_ids(&state, "liliana_chosen").is_empty());
    // Each list a slice of three ids, never an array of its own length: an
    // array of none is of no size, which Kani does not follow, and each
    // proof of an empty list ran out of memory.
    let (befores, afters) = ([ObjectId(10), ObjectId(11), ObjectId(12)], [ObjectId(20), ObjectId(21), ObjectId(22)]);
    let (before, after) = (&befores[..BEFORE], &afters[..AFTER]);
    store_ids(&mut state, "liliana_chosen", before);
    assert!(load_ids(&state, "liliana_chosen") == before && keeps_the_other_entry(&state, BEFORE + 1));
    store_ids(&mut state, "liliana_chosen", after);
    assert!(load_ids(&state, "liliana_chosen") == after && keeps_the_other_entry(&state, AFTER + 1));
    clear_ids(&mut state, "liliana_chosen");
    assert!(load_ids(&state, "liliana_chosen").is_empty() && keeps_the_other_entry(&state, 0));
    std::mem::forget(state);
}

macro_rules! each_pair {
    ($($name:ident: $before:literal, $after:literal;)*) => {$(
        action_proof! {
            /// [`storing_over_then_clearing`], for one pair of lengths.
            fn $name() {
                storing_over_then_clearing::<$before, $after>();
            }
        }
    )*};
}

each_pair! {
    a_kept_list_of_none_then_none: 0, 0;
    a_kept_list_of_none_then_one: 0, 1;
    a_kept_list_of_none_then_two: 0, 2;
    a_kept_list_of_none_then_three: 0, 3;
    a_kept_list_of_one_then_none: 1, 0;
    a_kept_list_of_one_then_one: 1, 1;
    a_kept_list_of_one_then_two: 1, 2;
    a_kept_list_of_one_then_three: 1, 3;
    a_kept_list_of_two_then_none: 2, 0;
    a_kept_list_of_two_then_one: 2, 1;
    a_kept_list_of_two_then_two: 2, 2;
    a_kept_list_of_two_then_three: 2, 3;
    a_kept_list_of_three_then_none: 3, 0;
    a_kept_list_of_three_then_one: 3, 1;
    a_kept_list_of_three_then_two: 3, 2;
    a_kept_list_of_three_then_three: 3, 3;
}

action_proof! {
    /// Two lists under two prefixes, as Liliana of the Veil keeps the
    /// players yet to choose and the cards chosen: each reads back as
    /// stored, storing or clearing one leaves the other as it was.
    fn two_kept_lists_are_kept_apart() {
        let mut state = state_with_another_entry();
        let queue = [ObjectId(1)];
        let chosen = [ObjectId(10), ObjectId(11)];
        store_ids(&mut state, "liliana_queue", &queue);
        store_ids(&mut state, "liliana_chosen", &chosen);
        assert!(load_ids(&state, "liliana_queue") == queue && load_ids(&state, "liliana_chosen") == chosen);
        store_ids(&mut state, "liliana_queue", &[]);
        assert!(load_ids(&state, "liliana_queue").is_empty() && load_ids(&state, "liliana_chosen") == chosen);
        clear_ids(&mut state, "liliana_chosen");
        assert!(load_ids(&state, "liliana_queue").is_empty() && load_ids(&state, "liliana_chosen").is_empty());
        clear_ids(&mut state, "liliana_queue");
        assert!(keeps_the_other_entry(&state, 0));
        std::mem::forget(state);
    }
}

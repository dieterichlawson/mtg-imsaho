//! The actions that take cards off the top of a library and put them into
//! one, which a card proof's `record_draw`, `record_mill` and
//! `record_to_library` stand in for: `engine::draw_cards` (CR 121),
//! `engine::mill_cards` (701.17a) and `GameState::put_into_library`.

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{act, acted, asked_for, blank_board, Act, Snapshot};
use crate::replacement::ReplaceableEvent;
use crate::state::{GameState, LibraryPosition};
use crate::types::Zone;

/// Who was asked about drawing from an empty library, and how often.
static mut EMPTY_DRAWS: (Option<PlayerId>, usize) = (None, 0);

/// Stands in for `replacement::apply`: drawing asks it one question, about
/// a draw from an empty library, which is recorded. What a replacement
/// does then is the card's own (Laboratory Maniac's proof).
#[allow(dead_code)] // named only by `kani::stub`
fn record_empty_draw(_: &mut GameState, event: ReplaceableEvent, _: &CardRegistry) -> Option<ReplaceableEvent> {
    let ReplaceableEvent::DrawsFromEmptyLibrary { player } = event else {
        panic!("drawing asks about nothing but an empty library");
    };
    // SAFETY: a proof runs on one thread, and only this module names it.
    unsafe {
        let asked = &mut *std::ptr::addr_of_mut!(EMPTY_DRAWS);
        *asked = (Some(player), asked.1 + 1);
    }
    Some(event)
}

/// Three cards of `who`'s, the first `size` of them (any number up to
/// three) their library, top first, and the rest in exile: the same
/// objects on every board, so that no object's place in the game is a
/// question for the solver.
fn a_library(state: &mut GameState, blank: crate::ids::CardId, who: PlayerId) -> ([ObjectId; 3], usize) {
    let size: usize = kani::any();
    kani::assume(size <= 3);
    let cards: [ObjectId; 3] = core::array::from_fn(|i| {
        state.create_object(blank, who, if i < size { Zone::Library } else { Zone::Exile }, None, None)
    });
    for (i, card) in cards.iter().enumerate() {
        if i < size {
            // A branch on the player, not an index by it: `any_object` says why.
            if who == PlayerId(0) {
                state.players[0].library_order.push(*card);
            } else {
                state.players[1].library_order.push(*card);
            }
        }
    }
    (cards, size)
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
#[kani::stub(crate::replacement::apply, record_empty_draw)]
/// "Draw N cards" (CR 121.1, 121.2): N times, the top card of that
/// player's library is put into their hand, and each draw is announced,
/// in order. A draw from an empty library draws nothing, marks the player
/// as having tried (which CR 704.5b makes a loss) and asks, once, whether
/// anything replaces it (614.1, Laboratory Maniac); no draw is tried after
/// it. What comes back is how many were drawn. For either player, a
/// library of up to three cards and N up to four; nothing else changes.
fn drawing_n_puts_the_top_n_cards_into_the_hand_in_order() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let (cards, size) = a_library(&mut state, blank, who);
    let n: usize = kani::any();
    kani::assume(n <= 4);
    let before = Snapshot::of(&state);

    let drawn = crate::engine::draw_cards(&mut state, who, n, &registry);

    let k = n.min(size);
    assert!(drawn == k);
    assert!(acted() == k && (0..k).all(|i| Some(&Act::Move(cards[i], Zone::Hand)) == act(i)));
    assert!(state.events.len() == k && (0..k).all(|i| matches!(state.events[i],
        GameEvent::CardDrawn { player, object } if player == who && object == cards[i])));
    for p in [PlayerId(0), PlayerId(1)] {
        assert!(state.get_player(p).has_drawn_from_empty == (p == who && n > size));
    }
    // SAFETY: as in `record_empty_draw`.
    let asked = unsafe { *std::ptr::addr_of!(EMPTY_DRAWS) };
    assert!(asked == if n > size { (Some(who), 1) } else { (None, 0) });
    assert!(Snapshot::of(&state).without_events() == before.with_library(who, &cards[k..size]).without_events());
    kani::cover!(size == 2 && n == 4, "the library runs out part way");
    std::mem::forget((registry, state));
}

/// Stands in for `GameState::move_object`: recorded, and, for a card in a
/// library, gone from its owner's library order, as `move_object` takes it
/// (its own proof says so); nothing else. A library lists a card once, so
/// taking out the one place it is is `move_object`'s `retain`, which CBMC
/// follows far less cheaply.
#[allow(dead_code)] // named only by `kani::stub`
fn move_keeping_library_order(state: &mut GameState, id: ObjectId, to: Zone, registry: &CardRegistry) -> bool {
    crate::proofs::cards::record_move(state, id, to, registry);
    let card = state.get_object(id).expect("a card in the game");
    if card.zone != Zone::Library {
        return true;
    }
    let owner = card.owner;
    // A branch on the player, not an index by it: `any_object` says why.
    let library = if owner == PlayerId(0) { &mut state.players[0].library_order } else { &mut state.players[1].library_order };
    if let Some(i) = library.iter().position(|c| *c == id) {
        library.remove(i);
    }
    true
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, move_keeping_library_order)]
/// "Mill N cards" (CR 701.17a): the top N cards of that player's library,
/// or all of them if there are fewer, are put into their graveyard one at a
/// time, top first, and are what comes back. For either player, a library
/// of up to three cards and N up to four; nothing else changes.
fn milling_n_puts_the_top_n_cards_into_the_graveyard_in_order() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let (cards, size) = a_library(&mut state, blank, who);
    let n: usize = kani::any();
    kani::assume(n <= 4);
    let before = Snapshot::of(&state);

    let milled = crate::engine::mill_cards(&mut state, who, n, "a source", &registry);

    let k = n.min(size);
    assert!(milled.as_slice() == &cards[..k]);
    let moves: [Act; 3] = core::array::from_fn(|i| Act::Move(cards[i], Zone::Graveyard));
    assert!(asked_for(&moves[..k]));
    assert!(Snapshot::of(&state) == before.with_library(who, &cards[k..size]));
    kani::cover!(size == 2 && n == 4, "the library runs out part way");
    std::mem::forget((registry, state, milled, moves));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, move_keeping_library_order)]
/// "Put it on top of its owner's library", or "on the bottom": it is moved
/// into that library and is then its top card, or its bottom one, with the
/// others in the order they were (CR 401.2: nothing else changes the
/// order of a library). A card already in the library moves
/// within it. For a card of either player's, in a library of up to three
/// cards or outside it, put on top or at the bottom; nothing else changes.
fn putting_a_card_into_a_library_puts_it_on_top_or_at_the_bottom() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let (cards, size) = a_library(&mut state, blank, who);
    let pick: usize = kani::any();
    kani::assume(pick < 3);
    let id = cards[pick];
    let top: bool = kani::any();
    let before = Snapshot::of(&state);

    state.put_into_library(id, if top { LibraryPosition::Top } else { LibraryPosition::Bottom }, &registry);

    let mut expected: Vec<ObjectId> = Vec::with_capacity(3);
    if top {
        expected.push(id);
    }
    for card in &cards[..size] {
        if *card != id {
            expected.push(*card);
        }
    }
    if !top {
        expected.push(id);
    }
    assert!(asked_for(&[Act::Move(id, Zone::Library)]));
    assert!(Snapshot::of(&state) == before.with_library(who, &expected));
    kani::cover!(pick < size && !top, "from the library to its bottom");
    std::mem::forget((registry, state, expected));
}

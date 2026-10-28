//! The actions that take cards off the top of a library and put them into
//! one, which a card proof's `record_draw`, `record_mill` and
//! `record_to_library` stand in for: `engine::draw_cards` (CR 121),
//! `engine::mill_cards` (701.17a) and `GameState::put_into_library`.
//!
//! Each is proved for libraries of none to three cards, a proof for each
//! size: a library whose length the solver leaves open is one whose cards
//! move by a distance it leaves open as one is taken out or put in, which
//! CBMC encodes over the whole buffer, and milling such a library ran it
//! out of memory.

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::proofs::cards::{asked_each, asked_for, blank_board, Act, Snapshot};
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

/// `SIZE` cards of `who`'s, their library, top first.
fn a_library<const SIZE: usize>(state: &mut GameState, blank: CardId, who: PlayerId) -> [ObjectId; SIZE] {
    let cards: [ObjectId; SIZE] = core::array::from_fn(|_| state.create_object(blank, who, Zone::Library, None, None));
    for card in &cards {
        // A branch on the player, not an index by it: `any_object` says why.
        if who == PlayerId(0) {
            state.players[0].library_order.push(*card);
        } else {
            state.players[1].library_order.push(*card);
        }
    }
    cards
}

/// "Draw N cards" (CR 121.1, 121.2): N times, the top card of that
/// player's library is put into their hand, and each draw is announced,
/// in order. A draw from an empty library draws nothing, marks the player
/// as having tried (which CR 704.5b makes a loss) and asks, once, whether
/// anything replaces it (614.1, Laboratory Maniac); no draw is tried after
/// it. What comes back is how many were drawn. For either player, a
/// library of `SIZE` cards and N up to four; nothing else changes.
fn drawing<const SIZE: usize>() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let cards = a_library::<SIZE>(&mut state, blank, who);
    let n: usize = kani::any();
    kani::assume(n <= 4);
    let before = Snapshot::of(&state);

    let drawn = crate::engine::draw_cards(&mut state, who, n, &registry);

    let k = n.min(SIZE);
    assert!(drawn == k);
    assert!(asked_each(k, |i| Act::Move(cards[i], Zone::Hand)));
    assert!(state.events.len() == k && (0..k).all(|i| matches!(state.events[i],
        GameEvent::CardDrawn { player, object } if player == who && object == cards[i])));
    for p in [PlayerId(0), PlayerId(1)] {
        assert!(state.get_player(p).has_drawn_from_empty == (p == who && n > SIZE));
    }
    // SAFETY: as in `record_empty_draw`.
    let asked = unsafe { *std::ptr::addr_of!(EMPTY_DRAWS) };
    assert!(asked == if n > SIZE { (Some(who), 1) } else { (None, 0) });
    assert!(Snapshot::of(&state).without_events() == before.with_library(who, &cards[k..]).without_events());
    kani::cover!(n > SIZE, "the library runs out");
    std::mem::forget((registry, state));
}

/// "Mill N cards" (CR 701.17a): the top N cards of that player's library,
/// or all of them if there are fewer, are put into their graveyard one at a
/// time, top first, and are what comes back. For either player, a library
/// of `SIZE` cards and N up to four; nothing else changes.
fn milling<const SIZE: usize>() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let cards = a_library::<SIZE>(&mut state, blank, who);
    let n: usize = kani::any();
    kani::assume(n <= 4);
    let before = Snapshot::of(&state);

    let milled = crate::engine::mill_cards(&mut state, who, n, "a source", &registry);

    let k = n.min(SIZE);
    assert!(milled.as_slice() == &cards[..k]);
    assert!(asked_each(k, |i| Act::Move(cards[i], Zone::Graveyard)));
    assert!(Snapshot::of(&state) == before.with_library(who, &cards[k..]));
    kani::cover!(n > SIZE, "the library runs out");
    std::mem::forget((registry, state, milled));
}

/// "Put it on top of its owner's library", or "on the bottom": it is moved
/// into that library and is then its top card, or its bottom one, with the
/// others in the order they were (CR 401.2: nothing else changes the
/// order of a library). A card already in the library moves within it.
/// For a card of either player's, one of a library of `SIZE` cards or one
/// in exile, put on top or at the bottom; nothing else changes.
fn putting<const SIZE: usize>() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let cards = a_library::<SIZE>(&mut state, blank, who);
    let outside = state.create_object(blank, who, Zone::Exile, None, None);
    let pick: usize = kani::any();
    kani::assume(pick <= SIZE);
    let id = if pick < SIZE { cards[pick] } else { outside };
    let top: bool = kani::any();
    let before = Snapshot::of(&state);

    state.put_into_library(id, if top { LibraryPosition::Top } else { LibraryPosition::Bottom }, &registry);

    let mut expected: Vec<ObjectId> = Vec::with_capacity(SIZE + 1);
    if top {
        expected.push(id);
    }
    for card in &cards {
        if *card != id {
            expected.push(*card);
        }
    }
    if !top {
        expected.push(id);
    }
    assert!(asked_for(&[Act::Move(id, Zone::Library)]));
    assert!(Snapshot::of(&state) == before.with_library(who, &expected));
    kani::cover!(pick < SIZE && !top, "from the library to its bottom");
    std::mem::forget((registry, state, expected));
}

macro_rules! for_each_size {
    ($body:ident, $move:path, [$($name:ident: $size:literal),*]) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::state::GameState::move_object, $move)]
        #[kani::stub(crate::replacement::apply, record_empty_draw)]
        /// The proof above it, for a library of that many cards.
        fn $name() {
            $body::<$size>();
        }
    )*};
}

for_each_size!(drawing, crate::proofs::cards::record_move, [
    drawing_from_an_empty_library: 0,
    drawing_from_a_library_of_one: 1,
    drawing_from_a_library_of_two: 2,
    drawing_from_a_library_of_three: 3
]);

for_each_size!(milling, move_keeping_library_order, [
    milling_an_empty_library: 0,
    milling_a_library_of_one: 1,
    milling_a_library_of_two: 2,
    milling_a_library_of_three: 3
]);

for_each_size!(putting, move_keeping_library_order, [
    putting_a_card_into_an_empty_library: 0,
    putting_a_card_into_a_library_of_one: 1,
    putting_a_card_into_a_library_of_two: 2,
    putting_a_card_into_a_library_of_three: 3
]);

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
/// Milling one card (CR 701.17a) is putting it into its owner's graveyard,
/// which `move_object` does (its own proof says how): for a card of either
/// player's, the one move, and nothing else.
fn milling_one_card_puts_it_into_its_owners_graveyard() {
    let (registry, mut state, blank) = blank_board();
    let card = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Library, None, None);
    let before = Snapshot::of(&state);
    crate::engine::mill_one(&mut state, card, &registry);
    assert!(asked_for(&[Act::Move(card, Zone::Graveyard)]) && Snapshot::of(&state) == before);
    std::mem::forget((registry, state));
}

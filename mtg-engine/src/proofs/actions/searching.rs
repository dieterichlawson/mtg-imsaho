//! Searching a library, which a card proof's `record_search` stands in
//! for: `helpers::search_library`, and what its answer does,
//! `helpers::finish_library_search` (CR 701.19). The moves and the
//! shuffle are `move_object`'s and `shuffle_library`'s, recorded here
//! (their own proofs say what they do).

use crate::actions::Target;
use crate::cards::helpers;
use crate::ids::PlayerId;
use crate::proofs::cards::{acted, asked_for, blank_board, Act, Snapshot};
use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::cards::helpers::shuffle_library, crate::proofs::cards::record_shuffle)]
/// "Search your library for a card with a quality" (CR 701.19): the
/// searcher is asked to choose among the cards of the library with that
/// quality, each card offered once however many copies there are (the
/// copies are alike: which one is found makes no difference); a search
/// the player may decline is asked with declining allowed, and its answer
/// finishes the search as `finish_library_search` does. A search the
/// player must make, with nothing to find, is a library shuffled (701.19b:
/// they need not find anything) and nothing else. A search they may
/// decline, with nothing to find, is declined for them, and nothing at all
/// happens: the engine does not offer searching for nothing, which the
/// player could otherwise choose for the shuffle. For either player, up
/// to two candidates of the same card or of two, to any zone, tapped or
/// not. Nothing else changes.
fn searching_asks_among_the_cards_with_the_quality() {
    let (mut registry, mut state, blank) = blank_board();
    let other = registry.register(Box::new(crate::proofs::cards::OtherBlank));
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(blank, who, Zone::Battlefield, None, None);
    let same: bool = kani::any();
    let found = [state.create_object(blank, who, Zone::Library, None, None),
        state.create_object(if same { blank } else { other }, who, Zone::Library, None, None)];
    let n: usize = kani::any();
    kani::assume(n <= 2);
    let mut candidates = Vec::with_capacity(2);
    for id in &found[..n] {
        candidates.push(*id);
    }
    let destination = if kani::any() { Zone::Hand } else { Zone::Battlefield };
    let (tapped, optional): (bool, bool) = (kani::any(), kani::any());
    let before = Snapshot::of(&state);

    helpers::search_library(&mut state, source, who, candidates, destination, tapped, optional, "a search");

    let offered = if n == 2 && same { 1 } else { n };
    if n == 0 {
        if optional {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
        } else {
            assert!(asked_for(&[Act::Shuffle(who)]) && Snapshot::of(&state) == before);
        }
    } else {
        assert!(acted() == 0 && Snapshot::of(&state) == before.with_asking(true));
        let Some(AwaitingAction::ResolutionChoice { player, source: s, choice }) = &state.awaiting_action else { panic!("asks") };
        assert!(*player == who && *s == source);
        if optional {
            assert!(matches!(choice, ResolutionChoiceKind::ChooseTarget {
                options, optional: true, effect: PendingEffect::FinishLibrarySearch { searcher, destination: d, tapped: t }, ..
            } if options.len() == offered && (0..offered).all(|i| options[i] == Target::Object(found[i]))
                && *searcher == who && *d == destination && *t == tapped));
        } else {
            assert!(matches!(choice, ResolutionChoiceKind::ChooseFromLibrary {
                options, searcher, source_id, destination: d, tapped: t, ..
            } if options.as_slice() == &found[..offered] && *searcher == who && *source_id == source && *d == destination && *t == tapped));
        }
    }
    kani::cover!(n == 2 && same && !optional, "two copies, offered once");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
#[kani::stub(crate::state::GameState::arrives_tapped, crate::proofs::cards::record_arrives_tapped)]
#[kani::stub(crate::cards::helpers::shuffle_library, crate::proofs::cards::record_shuffle)]
/// The card found is put where the search says (CR 701.19a), onto the
/// battlefield tapped if it says so (614.1c: it arrives tapped, nothing
/// taps it), and then the library is shuffled. Nothing else is done to
/// the card: whether it is summoning sick is what entering made it
/// (`move_object` makes a permanent that enters so, 302.6). For either
/// player, to the hand or the battlefield, tapped or not. Nothing else
/// changes.
fn finishing_a_search_puts_the_card_there_then_shuffles() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let found = state.create_object(blank, who, Zone::Library, Some(2), Some(2));
    let sick: bool = kani::any();
    state.get_object_mut(found).expect("just created").summoning_sick = sick;
    let destination = if kani::any() { Zone::Hand } else { Zone::Battlefield };
    let tapped: bool = kani::any();
    let before = Snapshot::of(&state);

    helpers::finish_library_search(&mut state, who, found, destination, tapped, &registry);

    if destination == Zone::Battlefield && tapped {
        assert!(asked_for(&[Act::Move(found, Zone::Battlefield), Act::ArrivesTapped { first: found, count: 1 }, Act::Shuffle(who)]));
    } else {
        assert!(asked_for(&[Act::Move(found, destination), Act::Shuffle(who)]));
    }
    assert!(state.get_object(found).expect("on the board").summoning_sick == sick);
    assert!(Snapshot::of(&state) == before);
    std::mem::forget((registry, state));
}

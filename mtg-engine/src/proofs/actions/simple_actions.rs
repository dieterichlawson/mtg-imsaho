//! The actions that change one thing: a player's life total (`change_life`
//! and the four other ways a card asks for it), a permanent's tapped
//! status (`tap` and `untap`, with their logging forms), the counters on a
//! permanent (`add_counters`, `remove_counters`), and a card's place in a
//! hand (`discard_card`). A card proof's recorder stands in for each.

use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, blank_board, new_game, Act, Snapshot};
use crate::types::{CounterType, Zone};

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Gain N life" and "lose N life" (CR 119.3): that player's total moves
/// by N and no other's does. A change of 0 is no life gained or lost
/// (119.10) and announces nothing; any other announces itself once, with
/// the totals either side. For every total and change an `i16` holds, and
/// each way a card asks: `change_life`, `change_life_for`,
/// `change_life_quiet`, `gain_life` and `lose_life`. Nothing else changes.
fn a_life_change_moves_that_players_total_and_no_others() {
    let mut state = new_game();
    let totals: [i16; 2] = [kani::any(), kani::any()];
    state.players[0].life = i32::from(totals[0]);
    state.players[1].life = i32::from(totals[1]);
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let delta = i32::from(kani::any::<i16>());
    let old = i32::from(totals[usize::from(who.0)]);
    let before = Snapshot::of(&state);

    match kani::any::<u8>() % 5 {
        0 => state.change_life(who, delta),
        1 => state.change_life_for(who, delta, "a source"),
        2 => state.change_life_quiet(who, delta),
        3 => {
            kani::assume(delta >= 0);
            state.gain_life(who, delta);
        }
        _ => {
            kani::assume(delta <= 0);
            state.lose_life(who, -delta);
        }
    }

    for p in [PlayerId(0), PlayerId(1)] {
        let total = i32::from(totals[usize::from(p.0)]);
        assert!(state.get_player(p).life == if p == who { total + delta } else { total });
    }
    if delta == 0 {
        assert!(state.events.is_empty());
    } else {
        assert!(matches!(state.events.as_slice(),
            [GameEvent::LifeChanged { player, old: o, new_life }] if *player == who && *o == old && *new_life == old + delta));
    }
    assert!(Snapshot::of(&state).without_events() == before.with_life(who, old + delta).without_events());
    kani::cover!(delta < 0 && who == PlayerId(1), "player 1 loses life");
    std::mem::forget(state);
}

/// An object of the blank card's, either player's, in any of four zones,
/// tapped or not; and the object an action is asked about: that one, or
/// one that is no longer in the game.
fn one_object() -> (crate::cards::CardRegistry, crate::state::GameState, ObjectId, ObjectId, Zone, bool) {
    let (registry, mut state, blank) = blank_board();
    let zone = match kani::any::<u8>() % 4 {
        0 => Zone::Battlefield,
        1 => Zone::Graveyard,
        2 => Zone::Hand,
        _ => Zone::Exile,
    };
    let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), zone, None, None);
    let tapped: bool = kani::any();
    state.get_object_mut(id).expect("just created").tapped = tapped;
    let asked = if kani::any() { id } else { ObjectId(id.0 + 1) };
    (registry, state, id, asked, zone, tapped)
}

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Tap" (CR 701.21a, 110.5): only an untapped permanent is tapped.
/// Tapping one taps it and announces it, once, and says it did; anything
/// else (a tapped permanent, a card in a graveyard, hand or exile, an
/// object no longer in the game) is left as it was, unannounced, and the
/// answer is that nothing was tapped. Through `tap` and `tap_for`.
fn tapping_taps_exactly_an_untapped_permanent() {
    let (registry, mut state, id, asked, zone, was_tapped) = one_object();
    let before = Snapshot::of(&state);

    let tapped = if kani::any() { state.tap(asked) } else { state.tap_for(asked, "a source") };

    let taps = asked == id && zone == Zone::Battlefield && !was_tapped;
    assert!(tapped == taps);
    if taps {
        assert!(matches!(state.events.as_slice(), [GameEvent::Tapped { object }] if *object == id));
        assert!(Snapshot::of(&state).without_events() == before.with_tapped(id, true).without_events());
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(taps, "a permanent is tapped");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Untap" (CR 701.21b, 110.5): only a tapped permanent is untapped.
/// Untapping one untaps it and announces it, once, and says it did;
/// anything else is left as it was, unannounced, and the answer is that
/// nothing was untapped. Through `untap` and `untap_for`.
fn untapping_untaps_exactly_a_tapped_permanent() {
    let (registry, mut state, id, asked, zone, was_tapped) = one_object();
    let before = Snapshot::of(&state);

    let untapped = if kani::any() { state.untap(asked) } else { state.untap_for(asked, "a source") };

    let untaps = asked == id && zone == Zone::Battlefield && was_tapped;
    assert!(untapped == untaps);
    if untaps {
        assert!(matches!(state.events.as_slice(), [GameEvent::Untapped { object }] if *object == id));
        assert!(Snapshot::of(&state).without_events() == before.with_tapped(id, false).without_events());
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(untaps, "a permanent is untapped");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
/// "Discard a card" (CR 701.8a): a card in its owner's hand is put into
/// a graveyard (which is its owner's, 404.1, as `move_object` says) and
/// the discard announced once, naming that player; anything else (a card
/// in a graveyard or exile, a permanent, an object no longer in the game)
/// is neither moved nor announced. Through `discard_card` and
/// `discard_card_for`.
fn discarding_moves_exactly_a_card_in_a_hand_to_a_graveyard() {
    let (registry, mut state, id, asked, zone, _) = one_object();
    let owner = state.get_object(id).expect("just created").owner;
    let before = Snapshot::of(&state);

    if kani::any() {
        state.discard_card(asked, &registry);
    } else {
        state.discard_card_for(asked, "a source", &registry);
    }

    if asked == id && zone == Zone::Hand {
        assert!(asked_for(&[Act::Move(id, Zone::Graveyard)]));
        assert!(matches!(state.events.as_slice(),
            [GameEvent::Discarded { player, object }] if *player == owner && *object == id));
        assert!(Snapshot::of(&state).without_events() == before.without_events());
    } else {
        assert!(acted() == 0);
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(asked == id && zone == Zone::Hand, "a card is discarded");
    std::mem::forget((registry, state));
}

/// Any kind of counter the engine has.
fn any_counter_kind() -> CounterType {
    match kani::any::<u8>() % 6 {
        0 => CounterType::PlusOnePlusOne,
        1 => CounterType::MinusOneMinusOne,
        2 => CounterType::Loyalty,
        3 => CounterType::Slime,
        4 => CounterType::Study,
        _ => CounterType::Hatchling,
    }
}

/// The object of [`one_object`] with up to three counters of one kind on
/// it, wherever it is: that kind, and how many.
fn with_counters(state: &mut crate::state::GameState, id: ObjectId) -> (CounterType, u32) {
    let (kind, n) = (any_counter_kind(), kani::any::<u32>());
    kani::assume(n <= 3);
    if n > 0 {
        state.get_object_mut(id).expect("just created").counters.insert(kind, n);
    }
    (kind, n)
}

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Put N counters on it" (CR 122.1): counters go on a permanent. On one,
/// N of that kind join those it has and no other kind changes; on anything
/// else (a card off the battlefield, an object no longer in the game), and
/// for N = 0, nothing changes at all. Through `add_counters` and
/// `add_counters_quiet`, which says whether they landed; for every kind,
/// N up to 3, and up to 3 of any one kind there already.
fn counters_go_on_a_permanent_and_only_of_their_kind() {
    let (registry, mut state, id, asked, zone, _) = one_object();
    let (had, had_n) = with_counters(&mut state, id);
    let (kind, n) = (any_counter_kind(), kani::any::<u32>());
    kani::assume(n <= 3);
    let before = Snapshot::of(&state);

    let lands = asked == id && zone == Zone::Battlefield && n > 0;
    if kani::any() {
        state.add_counters(asked, kind, n);
    } else {
        assert!(state.add_counters_quiet(asked, kind, n) == lands);
    }

    let old = if kind == had { had_n } else { 0 };
    if lands {
        assert!(Snapshot::of(&state) == before.with_counters(id, kind, old + n));
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(lands && old > 0, "counters join counters of their kind");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Remove N counters from it", as a cost or an effect says: N of that
/// kind come off, or every one if it has fewer, and it is left with none
/// of the kind listed rather than a count of 0; no other kind changes.
/// For every kind, N up to 4, and up to 3 of any one kind there, on an
/// object anywhere or no longer in the game.
fn removing_counters_removes_up_to_that_many_of_their_kind() {
    let (registry, mut state, id, asked, _, _) = one_object();
    let (had, had_n) = with_counters(&mut state, id);
    let (kind, n) = (any_counter_kind(), kani::any::<u32>());
    kani::assume(n <= 4);
    let before = Snapshot::of(&state);

    state.remove_counters(asked, kind, n);

    if asked == id && kind == had {
        assert!(Snapshot::of(&state) == before.with_counters(id, kind, had_n.saturating_sub(n)));
        assert!(had_n > n || !state.get_object(id).expect("still there").counters.contains_key(&kind));
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(asked == id && kind == had && had_n > 0 && had_n <= n, "every counter of the kind comes off");
    std::mem::forget((registry, state));
}

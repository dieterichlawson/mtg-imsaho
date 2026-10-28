//! The actions that change one thing: a player's life total (`change_life`
//! and the four other ways a card asks for it), a permanent's tapped
//! status (`tap` and `untap`, with their logging forms, and
//! `arrives_tapped`), the counters on a permanent (`add_counters`,
//! `remove_counters`), its regeneration shields
//! (`add_regeneration_shield`), what it is attached to (`attach`), who
//! controls it (`change_control`, `gain_control_while_source_controlled`),
//! its power and toughness as set in layer 7b (`set_base_pt`), a card's
//! place in a hand (`discard_card`, and `discard_cards` for N), a player's
//! mana pool (`add_mana`), whether a player has lost (`player_loses`) and
//! whether the game is over (`end_game`). A card proof's recorder stands
//! in for each.

use crate::events::{GameEvent, LossReason};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_each, asked_for, blank_board, new_game, Act, Snapshot};
use crate::state::{AwaitingAction, ControlEffect, GameResult, ResolutionChoiceKind, SetPtEffect};
use crate::types::{CounterType, ManaType, Zone};

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
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

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Attach it to that object" (CR 701.3a): it is taken from where it is,
/// the object or the player it was attached to (702.6c), and put onto that
/// object; attached to it already, nothing happens at all (701.3b). The
/// answer is whether anything changed, and nothing else does. For a
/// permanent on nothing, on either of two objects or on either player,
/// attached to either object, or asked about when no longer in the game.
fn attaching_puts_it_onto_that_object_from_wherever_it_was() {
    let (registry, mut state, blank) = blank_board();
    let hosts = [PlayerId(0), PlayerId(1)].map(|p| state.create_object(blank, p, Zone::Battlefield, None, None));
    let it = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
    let (on, on_player) = match kani::any::<u8>() % 5 {
        0 => (None, None),
        1 => (Some(hosts[0]), None),
        2 => (Some(hosts[1]), None),
        3 => (None, Some(PlayerId(0))),
        _ => (None, Some(PlayerId(1))),
    };
    let o = state.get_object_mut(it).expect("just created");
    o.attached_to = on;
    o.attached_to_player = on_player;
    let host = if kani::any() { hosts[0] } else { hosts[1] };
    let asked = if kani::any() { it } else { ObjectId(it.0 + 1) };
    let before = Snapshot::of(&state);

    let changed = state.attach(asked, host);

    let moves = asked == it && on != Some(host);
    assert!(changed == moves);
    if moves {
        assert!(Snapshot::of(&state) == before.with_attached(it, host));
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(moves && on_player.is_some(), "taken off a player");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Regenerate it" (CR 701.15a) sets up a shield on a permanent: it has
/// one more regeneration shield, and nothing else changes; anything else
/// (a card off the battlefield, which would carry the shield through a
/// graveyard and back, an object no longer in the game) gets none. For a
/// permanent with up to two shields already.
fn a_regeneration_shield_goes_on_a_permanent() {
    let (registry, mut state, id, asked, zone, _) = one_object();
    let shields = u32::from(kani::any::<u8>() % 3);
    state.get_object_mut(id).expect("just created").regeneration_shields = shields;
    let before = Snapshot::of(&state);

    state.add_regeneration_shield(asked);

    let gets = asked == id && zone == Zone::Battlefield;
    assert!(state.get_object(id).expect("still there").regeneration_shields == shields + u32::from(gets));
    assert!(Snapshot::of(&state) == before);
    kani::cover!(gets && shields > 0, "a second shield");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Enters tapped" (CR 614.1c), "put onto the battlefield tapped", "tapped
/// and attacking": the permanent is tapped, and since nothing tapped it,
/// no tap is announced; anything else (a card off the battlefield, an
/// object no longer in the game) is left as it was.
fn arriving_tapped_taps_a_permanent_unannounced() {
    let (registry, mut state, id, asked, zone, _) = one_object();
    let before = Snapshot::of(&state);

    state.arrives_tapped(asked);

    if asked == id && zone == Zone::Battlefield {
        assert!(Snapshot::of(&state) == before.with_tapped(id, true));
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Gain control of target permanent" (CR 110.2, 613.1b): a permanent
/// another player controls comes under the new controller's control, is
/// summoning sick for them (302.6), and the change is announced once,
/// naming both. One they control already, a card off the battlefield (whose
/// owner is its controller, 108.4) and an object no longer in the game are
/// left as they were, unannounced. For a permanent either player controls,
/// summoning sick or not, and either new controller.
fn control_changes_only_of_a_permanent_to_another_player() {
    let (registry, mut state, id, asked, zone, _) = one_object();
    let owner = state.get_object(id).expect("just created").owner;
    let controller = if zone == Zone::Battlefield && kani::any() { PlayerId(1 - owner.0) } else { owner };
    let sick: bool = kani::any();
    let o = state.get_object_mut(id).expect("just created");
    o.controller = controller;
    o.summoning_sick = sick;
    let to = PlayerId(u8::from(kani::any::<bool>()));
    let before = Snapshot::of(&state);

    state.change_control(asked, to);

    let changes = asked == id && zone == Zone::Battlefield && controller != to;
    let o = state.get_object(id).expect("still there");
    if changes {
        assert!(matches!(state.events.as_slice(),
            [GameEvent::ControlChanged { object, from, to: t }] if *object == id && *from == controller && *t == to));
        assert!(Snapshot::of(&state).without_events() == before.with_controller(id, to).without_events());
        assert!(o.summoning_sick);
    } else {
        assert!(Snapshot::of(&state) == before && o.summoning_sick == sick);
    }
    kani::cover!(changes && to == owner, "it goes back to its owner");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Gain control of target creature for as long as you control this"
/// (CR 611.2b, 613.1b): for a permanent and a source on the battlefield,
/// the effect is kept, naming both, the source's controller, and the
/// permanent's default controller to return it to (110.2a), with a
/// timestamp later than every one handed out, in place of an earlier one
/// for the same two (a second activation is the same effect again); and,
/// it being the latest, the permanent's controller is then the source's,
/// as `change_control` makes it. For either player controlling each, with
/// or without such an effect already, and alongside one from another
/// source. (The card asks only while it controls the source, which its
/// own proof shows; the duration is then `expire_control_effects`'s.)
fn gaining_control_while_controlling_a_source_keeps_one_effect_and_takes_control() {
    let (registry, mut state, blank) = blank_board();
    let [owner, sc] = [kani::any::<bool>(), kani::any::<bool>()].map(|b| PlayerId(u8::from(b)));
    let it = state.create_object(blank, owner, Zone::Battlefield, None, None);
    let source = state.create_object(blank, sc, Zone::Battlefield, None, None);
    let elsewhere = state.create_object(blank, sc, Zone::Battlefield, None, None);
    let controller = if kani::any() { PlayerId(1 - owner.0) } else { owner };
    state.get_object_mut(it).expect("just created").controller = controller;
    let issued: u64 = kani::any();
    kani::assume(issued < u64::MAX);
    state.next_effect_timestamp = issued;
    let effect = |source, timestamp| ControlEffect {
        object: it, controller, original_controller: owner, source, source_controller: controller, timestamp,
    };
    let (t0, t1): (u64, u64) = (kani::any(), kani::any());
    kani::assume(t0 <= issued && t1 <= issued);
    let again: bool = kani::any();
    state.control_effects.reserve(3);
    state.control_effects.push(effect(elsewhere, t0));
    if again {
        state.control_effects.push(effect(source, t1));
    }
    let before = Snapshot::of(&state);

    state.gain_control_while_source_controlled(it, source, &registry);

    let effects = &state.control_effects;
    assert!(effects.len() == 2 && effects[0] == effect(elsewhere, t0));
    assert!(effects[1] == ControlEffect {
        object: it, controller: sc, original_controller: owner, source, source_controller: sc, timestamp: issued + 1,
    });
    let changes = controller != sc;
    if changes {
        assert!(matches!(state.events.as_slice(),
            [GameEvent::ControlChanged { object, from, to }] if *object == it && *from == controller && *to == sc));
        assert!(Snapshot::of(&state).without_events() == before.with_controller(it, sc).without_events());
    } else {
        assert!(Snapshot::of(&state) == before);
    }
    kani::cover!(again && changes, "activated again, it takes control");
    std::mem::forget((registry, state));
}

/// Any setting of power and toughness, on either of two objects, with a
/// timestamp up to `issued`.
fn a_setting(issued: u64) -> SetPtEffect {
    let timestamp: u64 = kani::any();
    kani::assume(timestamp <= issued);
    SetPtEffect {
        object: ObjectId(u64::from(kani::any::<bool>())),
        power: kani::any::<bool>().then(|| i32::from(kani::any::<i8>())),
        toughness: kani::any::<bool>().then(|| i32::from(kani::any::<i8>())),
        timestamp,
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// "Its toughness becomes N" (CR 613.4b, layer 7b): the setting is kept
/// for that permanent with a timestamp later than every one the game has
/// handed out (613.7a: of two, the later applies last and so wins), after
/// the settings there were, which are as they were; nothing else changes.
/// For up to two settings before it, of either of two permanents.
fn setting_power_and_toughness_keeps_the_latest_setting() {
    let mut state = new_game();
    let issued: u64 = kani::any();
    kani::assume(issued < u64::MAX);
    state.next_effect_timestamp = issued;
    let earlier = [a_setting(issued), a_setting(issued)];
    let k: usize = kani::any();
    kani::assume(k <= 2);
    state.set_pt_effects.reserve(3);
    for e in &earlier[..k] {
        state.set_pt_effects.push(e.clone());
    }
    let object = ObjectId(u64::from(kani::any::<bool>()));
    let (power, toughness) = (kani::any::<bool>().then_some(1), kani::any::<bool>().then_some(5));
    let before = Snapshot::of(&state);

    state.set_base_pt(object, power, toughness);

    let effects = &state.set_pt_effects;
    assert!(effects.len() == k + 1 && (0..k).all(|i| effects[i] == earlier[i]));
    let set = &effects[k];
    assert!(set.object == object && set.power == power && set.toughness == toughness);
    assert!(set.timestamp > issued && state.next_effect_timestamp == set.timestamp);
    assert!(Snapshot::of(&state) == before);
    std::mem::forget(state);
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// A player loses the game (CR 104.3): a player still in it is out of it,
/// with the reason kept and the loss announced once; a player who has
/// lost already is left as they were, the first reason standing, and
/// nothing is announced. The other player is unchanged. For either
/// player and four reasons.
fn a_player_loses_the_game_once() {
    let mut state = new_game();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let lost: bool = kani::any();
    // A branch on the player, not an index by it: `any_object` says why.
    let p = if who == PlayerId(0) { &mut state.players[0] } else { &mut state.players[1] };
    p.lost = lost;
    p.loss_reason = lost.then_some(LossReason::Conceded);
    let reason = match kani::any::<u8>() % 4 {
        0 => LossReason::LifeReachedZero,
        1 => LossReason::DrewFromEmptyLibrary,
        2 => LossReason::Conceded,
        _ => LossReason::Forfeited,
    };
    let before = Snapshot::of(&state);

    state.player_loses(who, reason);

    for q in [PlayerId(0), PlayerId(1)] {
        let p = state.get_player(q);
        if q != who {
            assert!(!p.lost && p.loss_reason.is_none());
        } else {
            assert!(p.lost && p.loss_reason == Some(if lost { LossReason::Conceded } else { reason }));
        }
    }
    if lost {
        assert!(state.events.is_empty());
    } else {
        assert!(matches!(state.events.as_slice(), [GameEvent::PlayerLost { player, reason: r }] if *player == who && *r == reason));
    }
    assert!(Snapshot::of(&state).without_events() == before.without_events());
    std::mem::forget(state);
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// The game ends (CR 104.1), once: a game in progress takes the result, a
/// win for either player or a draw (104.4a), announced once, and keeps the
/// game as it was then as its final state; a game over already keeps the
/// result it had, and nothing is announced.
fn a_game_ends_once() {
    let mut state = new_game();
    let over: bool = kani::any();
    if over {
        state.result = Some(GameResult::Draw);
    }
    let result = if kani::any() { GameResult::Winner(PlayerId(u8::from(kani::any::<bool>()))) } else { GameResult::Draw };
    let before = Snapshot::of(&state);

    state.end_game(result.clone());

    if over {
        assert!(state.result == Some(GameResult::Draw) && state.events.is_empty() && state.final_state.is_none());
    } else {
        assert!(state.result.as_ref() == Some(&result));
        assert!(matches!(state.events.as_slice(), [GameEvent::GameEnded { result: r }] if *r == result));
        let last = state.final_state.as_deref().expect("the game as it ended");
        assert!(last.result.as_ref() == Some(&result) && last.events.len() == 1);
    }
    assert!(Snapshot::of(&state).without_events() == before.without_events());
    std::mem::forget((state, result));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// "Add {G}" (CR 106.4): that much mana of that type joins what that
/// player's pool holds of it, the pool's other types and the other
/// player's pool are unchanged, and the mana is announced once. For either
/// player, every type, up to three of it in the pool before and up to
/// three added.
fn adding_mana_adds_it_to_that_players_pool() {
    let mut state = new_game();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let kind = match kani::any::<u8>() % 6 {
        0 => ManaType::White,
        1 => ManaType::Blue,
        2 => ManaType::Black,
        3 => ManaType::Red,
        4 => ManaType::Green,
        _ => ManaType::Colorless,
    };
    let (had, n) = (kani::any::<u32>(), kani::any::<u32>());
    kani::assume(had <= 3 && n <= 3);
    if had > 0 {
        // A branch on the player, not an index by it: `any_object` says why.
        let p = if who == PlayerId(0) { &mut state.players[0] } else { &mut state.players[1] };
        p.mana_pool.add(kind, had);
    }
    let before = Snapshot::of(&state);

    state.add_mana(who, kind, n);

    for q in [PlayerId(0), PlayerId(1)] {
        let pool = &state.get_player(q).mana_pool.mana;
        if q == who {
            assert!(pool.len() == 1 && pool.get(&kind) == Some(&(had + n)));
        } else {
            assert!(pool.is_empty());
        }
    }
    assert!(matches!(state.events.as_slice(),
        [GameEvent::ManaAdded { player, mana_type, amount }] if *player == who && *mana_type == kind && *amount == n));
    assert!(Snapshot::of(&state).without_events() == before.without_events());
    std::mem::forget(state);
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::discard_card_for, crate::proofs::cards::record_discard_for)]
#[kani::stub(crate::state::GameState::objects_in_zone, crate::proofs::cards::objects_in_zone_in_id_order)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// "Target player discards N cards" (CR 701.8a): the player chooses N
/// cards of their hand, and a player with no more than N discards them
/// all, unasked (as much as they can, 101.3), each of the hand in its
/// order; a player with more is asked, once, to choose a card of their
/// hand to discard, with N still to go. Discarding none, or from an empty
/// hand, does nothing. For either player, hands of up to two cards beside
/// a card in the other player's, and N up to three. Nothing else changes.
fn discarding_n_discards_the_hand_or_asks_for_the_cards() {
    let (registry, mut state, blank) = blank_board();
    let who = PlayerId(u8::from(kani::any::<bool>()));
    let held: usize = kani::any();
    kani::assume(held <= 2);
    // Both cards made, the ones not held in exile, so that every id is
    // known (`cards_in_zones` says why).
    let cards = [0, 1].map(|i| state.create_object(blank, who, if i < held { Zone::Hand } else { Zone::Exile }, None, None));
    state.create_object(blank, PlayerId(1 - who.0), Zone::Hand, None, None);
    let source = state.create_object(blank, who, Zone::Stack, None, None);
    let n: usize = kani::any();
    kani::assume(n <= 3);
    let before = Snapshot::of(&state);

    crate::engine::discard_cards(&mut state, who, n, source, "a source", &registry);

    if n == 0 || held == 0 {
        assert!(acted() == 0 && Snapshot::of(&state) == before);
    } else if held <= n {
        assert!(asked_each(held, |i| Act::Discard(cards[i])) && Snapshot::of(&state) == before);
    } else {
        assert!(acted() == 0 && Snapshot::of(&state) == before.with_asking(true));
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source: s, choice: ResolutionChoiceKind::ChooseCardFromHand {
                player: from, cards: offered, discard_immediately: true, remaining, ..
            },
        }) if *player == who && *s == source && *from == who && offered.as_slice() == &cards[..held] && *remaining == n));
    }
    kani::cover!(held == 2 && n == 1, "asked to choose");
    std::mem::forget((registry, state));
}

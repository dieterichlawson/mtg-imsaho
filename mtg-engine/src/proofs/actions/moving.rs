//! `GameState::move_object`, which a card proof's `record_move` stands in
//! for, and `move_object_under_control`, which `record_move_under` does: a
//! card put from one zone into another.
//!
//! Each zone a card can leave has a proof of its own, for a move into any
//! zone: the zone it leaves decides what is on the board before the move
//! (a permanent's tapped status and damage, a spell's cast), and a proof
//! that left it open would build every zone's board at once.
//!
//! The board is the blank card's, which prints no replacement effect and no
//! choice to make as it enters (CR 614.12): what a card that prints one
//! does as something enters is that card's own proof. The replacement
//! layer's answer for an entering permanent, `replacement::for_entering`,
//! stands in as the event unchanged, which is what it answers on such a
//! board (`entering_with_nothing_to_replace_it_is_unchanged`, below): run
//! through it here, the copy it might decide was one CBMC left open, and
//! becoming a copy was two thirds of the proof's work. Combat and the
//! turn's effects are empty here: `remove_from_combat` and the retain over
//! `until_end_of_turn` are not reached with anything to remove.

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::PlayerId;
use crate::proofs::cards::{blank_board, Snapshot};
use crate::replacement::EnteringPermanent;
use crate::state::{ControlEffect, EnterAsCopyChoice, GameState, SetPtEffect};
use crate::types::{CardType, Color, CounterType, Zone};

/// Stands in for `replacement::for_entering`: the event as it was.
#[allow(dead_code)] // named only by `kani::stub`
fn unchanged(_: &mut GameState, entering: EnteringPermanent, _: &CardRegistry) -> EnteringPermanent {
    entering
}

/// Stands in for `GameState::become_copy_of`, which nothing here may call:
/// no permanent enters as a copy when nothing decided one. The proof holds
/// only if no path reaches it; CBMC, which cannot tell from the bytes of
/// an entering event that no copy was decided, would otherwise follow
/// becoming a copy of something into every effect a copy might drop.
#[allow(dead_code)] // named only by `kani::stub`
fn no_copy(_: &mut GameState, _: crate::ids::ObjectId, _: crate::ids::ObjectId, _: &CardRegistry) {
    panic!("a permanent entered as a copy with no copy decided");
}

/// Any zone.
fn any_zone() -> Zone {
    match kani::any::<u8>() {
        0 => Zone::Library,
        1 => Zone::Hand,
        2 => Zone::Battlefield,
        3 => Zone::Graveyard,
        4 => Zone::Stack,
        5 => Zone::Exile,
        _ => Zone::Command,
    }
}

/// A card put from `from` into any zone (CR 400.7). It is then in that
/// zone, the move counted and announced (a move within a zone too, which
/// is how `put_into_library` moves a card within its library). Its
/// owner's library lists it no longer if it left the library (CR 401.2;
/// a card moved within the library is put back where it goes by
/// `put_into_library`), and its owner's graveyard lists it on top if it
/// arrived there (404.2) and no longer if it left. A permanent that leaves
/// the battlefield is a new object (400.7): its owner controls it (108.4),
/// it is untapped and undamaged, with no counters, attached to nothing, and
/// for a card no longer has what effects gave it (colors, types,
/// legendary, power and toughness) while a token keeps what it was made
/// with (111.1); that it left is announced, with who controlled it last
/// (603.10c), and the control and set-power-and-toughness effects on it
/// end. A creature card put from a library into a graveyard is a mill
/// (701.17a) and is announced as one. A permanent that enters is summoning
/// sick (302.6), controlled by whoever put it there (110.2a) and announced.
/// A card put into a hand or a library forgets how it was cast; one put
/// anywhere but the battlefield or the stack forgets its X (107.3b), and
/// anywhere but the battlefield what it chose to enter as (614.12b). A
/// spell that leaves the stack is no longer the one resolving. Through
/// `move_object`, and `move_object_under_control` for a card put onto the
/// battlefield from elsewhere under either player's control. Nothing else
/// changes.
fn moving(from: Zone, who: PlayerId) {
    let (registry, mut state, blank) = blank_board();
    let other = PlayerId(1 - who.0);
    // Its owner's library and graveyard hold a card each besides it.
    let below = state.create_object(blank, who, Zone::Library, None, None);
    let under = state.create_object(blank, who, Zone::Graveyard, None, None);
    let creature: bool = kani::any();
    let pt = creature.then_some(2);
    let id = state.create_object(blank, who, from, pt, pt);
    let owner = &mut state.players[usize::from(who.0)];
    owner.library_order.push(below);
    owner.graveyard_order.push(under);
    if from == Zone::Library {
        owner.library_order.push(id);
    }
    if from == Zone::Graveyard {
        owner.graveyard_order.push(id);
    }

    // What an effect may have given it.
    let granted: bool = kani::any();
    let token = from == Zone::Battlefield && kani::any::<bool>();
    let controller = if from == Zone::Battlefield && kani::any::<bool>() { other } else { who };
    let cursed = (from == Zone::Battlefield && kani::any::<bool>()).then_some(other);
    let resolving = from == Zone::Stack && kani::any::<bool>();
    let obj = state.get_object_mut(id).expect("just created");
    obj.is_token = token;
    obj.controller = controller;
    if granted {
        obj.colors = vec![Color::Black];
        obj.card_types = vec![CardType::Artifact];
        obj.is_legendary = true;
    }
    if from == Zone::Battlefield {
        obj.tapped = kani::any();
        obj.summoning_sick = kani::any();
        obj.damage_marked = u32::from(kani::any::<u8>());
        obj.regeneration_shields = u32::from(kani::any::<u8>());
        obj.attacked_on_turn = kani::any::<bool>().then_some(1);
        obj.attached_to_player = cursed;
        if kani::any() {
            obj.counters.insert(CounterType::PlusOnePlusOne, u32::from(kani::any::<u8>()));
        }
    }
    if from == Zone::Stack {
        obj.cast_with_flashback = kani::any();
        obj.cast_from_zone = kani::any::<bool>().then_some(Zone::Graveyard);
    }
    if matches!(from, Zone::Stack | Zone::Battlefield) {
        obj.x_value = kani::any::<bool>().then_some(3);
    }
    if resolving {
        state.resolving_spell = Some(id);
    }
    // An effect on it and one on another.
    let control = |object| ControlEffect {
        object, controller: other, original_controller: who, source: below, source_controller: other, timestamp: 0,
    };
    state.control_effects = vec![control(id), control(below)];
    let set_pt = |object| SetPtEffect { object, power: Some(1), toughness: None, timestamp: 0 };
    state.set_pt_effects = vec![set_pt(id), set_pt(below)];

    let to = any_zone();
    let entered = to == Zone::Battlefield && from != Zone::Battlefield;
    let under_control = (entered && kani::any::<bool>()).then(|| PlayerId(u8::from(kani::any::<bool>())));
    let before = Snapshot::of(&state);

    let moved = match under_control {
        Some(c) => state.move_object_under_control(id, to, c, &registry),
        None => state.move_object(id, to, &registry),
    };

    let left = from == Zone::Battlefield && to != Zone::Battlefield;
    let milled = from == Zone::Library && to == Zone::Graveyard && creature;
    assert!(moved);

    // The board.
    let mut expected = before.with_zone(id, to).with_library(who, &[below]);
    expected = if to == Zone::Graveyard {
        expected.with_graveyard(who, &[under, id])
    } else {
        expected.with_graveyard(who, &[under])
    };
    if left {
        expected = expected.as_a_new_object(id, who);
    }
    if let Some(c) = under_control {
        expected = expected.with_controller(id, c);
    }
    assert!(Snapshot::of(&state).without_events() == expected.without_events());

    // What was announced, in order.
    let events = state.events.as_slice();
    assert!(matches!(events.first(), Some(GameEvent::ObjectMoved { object, from: f, to: t })
        if *object == id && *f == from && *t == to));
    let mut n = 1;
    if left {
        assert!(matches!(events.get(n), Some(GameEvent::LeftBattlefield { object, to: t, last_controller })
            if *object == id && *t == to && *last_controller == controller));
        n += 1;
    }
    if milled {
        assert!(matches!(events.get(n), Some(GameEvent::CreatureCardMilled { object, milled_player })
            if *object == id && *milled_player == who));
        n += 1;
    }
    if entered {
        let c = under_control.unwrap_or(who);
        assert!(matches!(events.get(n), Some(GameEvent::EnteredBattlefield { object, controller })
            if *object == id && *controller == c));
        n += 1;
    }
    assert!(events.len() == n);

    // What the board's rows do not hold.
    let obj = state.get_object(id).expect("still in the game");
    assert!(obj.zone_change_count == 1);
    if entered {
        assert!(obj.summoning_sick && obj.base_controller == obj.controller);
        assert!(obj.card_state.is_empty() && obj.last_attached_to_player.is_none());
    }
    if left {
        assert!(!obj.summoning_sick && obj.last_controller == Some(controller));
        assert!(obj.regeneration_shields == 0 && obj.attacked_on_turn.is_none());
        assert!(obj.last_attached_to_player == cursed);
        if token {
            assert!(obj.colors.len() == usize::from(granted) && obj.power == pt && obj.toughness == pt);
        } else {
            assert!(obj.colors.is_empty() && obj.card_types.is_empty() && !obj.is_legendary);
            assert!(obj.power.is_none() && obj.toughness.is_none() && obj.name == "Blank");
        }
    }
    if matches!(to, Zone::Hand | Zone::Library) {
        assert!(!obj.cast_with_flashback && obj.cast_from_zone.is_none());
    }
    if !matches!(to, Zone::Battlefield | Zone::Stack) {
        assert!(obj.x_value.is_none());
    }
    if to != Zone::Battlefield {
        assert!(obj.entering_copy_choice == EnterAsCopyChoice::Unasked);
    }
    assert!(state.resolving_spell == (resolving && to == Zone::Stack).then_some(id));
    let lasting = if left { 1 } else { 2 };
    assert!(state.control_effects.len() == lasting && state.control_effects.last().is_some_and(|c| c.object == below));
    assert!(state.set_pt_effects.len() == lasting && state.set_pt_effects.last().is_some_and(|e| e.object == below));

    kani::cover!(left && to == Zone::Graveyard && !token, "a card dies");
    kani::cover!(milled, "a creature card is milled");
    kani::cover!(under_control == Some(other), "it enters under another's control");
    std::mem::forget((registry, state));
}

macro_rules! from_each_zone {
    ($($name:ident: $zone:ident),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::replacement::for_entering, unchanged)]
        #[kani::stub(crate::state::GameState::become_copy_of, no_copy)]
        /// The proof above it, for a card leaving that zone, of either
        /// player's. The owner is a constant in each call, not left to the
        /// solver: `move_object` reaches the owner's library and graveyard
        /// through an index by the owner, and a write through an index the
        /// solver leaves open makes every later read of either player a
        /// question of which one it was.
        fn $name() {
            if kani::any() {
                moving(Zone::$zone, PlayerId(0));
            } else {
                moving(Zone::$zone, PlayerId(1));
            }
        }
    )*};
}

from_each_zone! {
    moving_a_card_from_a_library: Library,
    moving_a_card_from_a_hand: Hand,
    moving_a_permanent_from_the_battlefield: Battlefield,
    moving_a_card_from_a_graveyard: Graveyard,
    moving_a_spell_from_the_stack: Stack,
    moving_a_card_from_exile: Exile,
    moving_a_card_from_the_command_zone: Command,
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
/// A permanent entering a board on which no card prints a replacement
/// effect (CR 614.1) enters as it was going to: the replacement layer
/// gives back the event it was asked about, from wherever the permanent
/// comes, under either player, tapped or not, with or without counters,
/// as a copy of something or not.
fn entering_with_nothing_to_replace_it_is_unchanged() {
    let (registry, mut state, blank) = blank_board();
    let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Graveyard, None, None);
    state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
    let mut counters = Vec::with_capacity(1);
    if kani::any() {
        counters.push((crate::types::CounterType::PlusOnePlusOne, 1));
    }
    let entering = EnteringPermanent {
        object: id,
        from: if kani::any() { Some(Zone::Graveyard) } else { None },
        controller: PlayerId(u8::from(kani::any::<bool>())),
        tapped: kani::any(),
        counters,
        copy_of: kani::any::<bool>().then_some(crate::ids::ObjectId(id.0 + 1)),
    };
    let asked = entering.clone();

    let answer = crate::replacement::for_entering(&mut state, entering, &registry);

    assert!(answer == asked);
    std::mem::forget((registry, state, answer, asked));
}

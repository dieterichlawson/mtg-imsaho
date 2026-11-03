//! State-based actions (`sba::check_state_based_actions`, CR 704): the
//! check repeated until nothing applies (704.3), one pass of it
//! (`sba::one_pass`), and each part of a pass, each its own proof, the
//! others standing in as any answer: the players' losses (704.5a-b), the
//! creatures' (704.5f-h: which die, `sba::creature_fates`, and what is
//! done to them, `sba::remove_creatures`), illegal attachments (704.5m-n),
//! counters annihilating (704.5q), state triggers (603.8), planeswalkers
//! without loyalty (704.5i), the legend rule (704.5j), tokens elsewhere
//! (704.5d) and the game's end (104.2a). Which of 704.5f-h applies to a
//! creature, given its toughness, damage and whether any was from a
//! deathtouch source, is `mtg_kernel::sba::creature_fate`, proved in
//! Verus; and what the parts ask of the board (whether an object is a
//! creature, its toughness, whether an attachment is illegal, a name) is
//! any answer here, each its own proof elsewhere.

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::events::{GameEvent, LossReason};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_each, asked_for, card_proof, Act, Blank};
use crate::state::{GameState, PendingEffect};
use crate::types::{CardType, CounterType, Keyword, Zone};

/// Whether each of the board's objects 1 to 3 is a creature, and each's
/// toughness: chosen once, the same after.
static mut CREATURE: [Option<bool>; 3] = [None; 3];
static mut TOUGHNESS: [Option<Option<i32>>; 3] = [None; 3];

fn index(id: ObjectId) -> usize {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=3).contains(&i), "an object this proof does not make");
    i - 1
}

/// Stands in for `GameState::is_creature`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn is_creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: a proof runs on one thread, and only this function names CREATURE.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(CREATURE) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::effective_toughness`: any toughness from -1
/// to 3, or none (an object it finds no toughness for), the same each time
/// it is asked.
#[allow(dead_code)] // named only by `kani::stub`
fn effective_toughness(_: &GameState, id: ObjectId, _: &CardRegistry) -> Option<i32> {
    // SAFETY: as in `is_creature`, for TOUGHNESS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(TOUGHNESS) };
    *answers[index(id)].get_or_insert_with(|| {
        let t: i32 = kani::any();
        kani::assume((-1..=3).contains(&t));
        kani::any::<bool>().then_some(t)
    })
}

/// Which creatures the pass removes (`creature_fates`, CR 704.5f-h): of
/// the board's objects on the battlefield that are creatures, in the
/// board's order, those the kernel's rule sends to the graveyard for their
/// toughness, and those it says are destroyed; the toughness asked of the
/// layers, or with none found there the object's own (each here 1). For
/// three objects, each either player's, on the battlefield or in a
/// graveyard, each a creature or not, of toughness -1 to 3 or none, with 0
/// to 4 damage marked, from a deathtouch source or not.
fn deciding() {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let mut on = [false; 3];
    let mut damage = [0u32; 3];
    let mut deathtouch = [false; 3];
    for i in 0..3 {
        on[i] = kani::any();
        let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
        let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), zone, Some(1), Some(1));
        let (d, dt): (u32, bool) = (kani::any(), kani::any());
        kani::assume(d <= 4);
        (damage[i], deathtouch[i]) = (d, dt);
        let o = state.get_object_mut(id).expect("just created");
        o.damage_marked = d;
        o.dealt_deathtouch_damage = dt;
    }

    let (zero, destroyed) = crate::sba::creature_fates(&state, &registry);

    let (mut z, mut d) = ([ObjectId(0); 3], [ObjectId(0); 3]);
    let (mut nz, mut nd) = (0, 0);
    for i in 0..3 {
        let id = ObjectId(i as u64 + 1);
        if !on[i] || !is_creature(&state, id, &registry) {
            continue;
        }
        let t = effective_toughness(&state, id, &registry).unwrap_or(1);
        match mtg_kernel::sba::creature_fate(t, damage[i], deathtouch[i]) {
            mtg_kernel::sba::Fate::ZeroToughness => {
                z[nz] = id;
                nz += 1;
            }
            mtg_kernel::sba::Fate::Destroyed => {
                d[nd] = id;
                nd += 1;
            }
            mtg_kernel::sba::Fate::Stays => {}
        }
    }
    assert!(zero.len() == nz && destroyed.len() == nd);
    assert!((0..nz).all(|k| zero[k] == z[k]) && (0..nd).all(|k| destroyed[k] == d[k]));
    std::mem::forget((registry, state, zero, destroyed));
}

card_proof! {
    #[kani::stub(crate::state::GameState::is_creature, is_creature)]
    #[kani::stub(crate::state::GameState::effective_toughness, effective_toughness)]
    /// [`deciding`].
    fn the_pass_removes_the_creatures_the_kernels_rule_says_die() {
        deciding();
    }
}

/// What the pass did to a destroyed creature: regenerated it, or
/// destroyed it, in order.
static mut DONE: [Option<(bool, ObjectId)>; 3] = [None; 3];
static mut DONE_N: usize = 0;

fn done(regenerated: bool, id: ObjectId) {
    // SAFETY: as in `is_creature`, for DONE and DONE_N.
    let (list, n) = unsafe { (&mut *std::ptr::addr_of_mut!(DONE), &mut *std::ptr::addr_of_mut!(DONE_N)) };
    list[*n] = Some((regenerated, id));
    *n += 1;
}

/// Stands in for `destruction::regenerate_sba` (what regeneration does is
/// `try_destroy`'s proof, which uses the same `regenerate`): recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn regenerate_sba(_: &mut GameState, id: ObjectId) {
    done(true, id);
}

/// Stands in for `destruction::destroy_sba` (`try_destroy`'s `destroy`, as
/// that proof shows it): recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn destroy_sba(_: &mut GameState, id: ObjectId, _: &CardRegistry) {
    done(false, id);
}

/// Stands in for `destruction::death_event` (`try_destroy`'s proof says
/// what it captures): a marker naming the creature.
#[allow(dead_code)] // named only by `kani::stub`
fn death_event(_: &GameState, id: ObjectId, _: Option<&CardRegistry>) -> Option<GameEvent> {
    Some(GameEvent::Tapped { object: id })
}

/// Whether each of the board's creatures has indestructible: any answer.
static mut INDESTRUCTIBLE: [Option<bool>; 3] = [None; 3];

#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    assert!(keyword == Keyword::Indestructible, "a keyword the pass does not ask");
    // SAFETY: as in `is_creature`, for INDESTRUCTIBLE.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(INDESTRUCTIBLE) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// What the pass does to the creatures it removes (`remove_creatures`, CR
/// 704.5f-h, 704.3): each of `zero` (toughness 0 or less), in order, has
/// its death captured and goes to its owner's graveyard, which
/// indestructible and regeneration do not stop, and a creature has died
/// this turn; then each of `destroyed` with indestructible is left, each
/// other with a regeneration shield is regenerated, and each other is
/// destroyed, in order. Whether anything was done is whether any of that
/// was. For each creature with indestructible or not and a shield or not.
fn removing(zero: &[u64], destroyed: &[u64]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let mut shielded = [false; 3];
    for s in &mut shielded {
        let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
        *s = kani::any();
        state.get_object_mut(id).expect("just created").regeneration_shields = u32::from(*s);
    }
    let events = state.events.len();
    let died = state.creature_died_this_turn;

    let acted = crate::sba::remove_creatures(&mut state,
        zero.iter().map(|&i| ObjectId(i)).collect(), destroyed.iter().map(|&i| ObjectId(i)).collect(), &registry);

    assert!(state.events.len() == events + zero.len());
    for (k, &i) in zero.iter().enumerate() {
        assert!(matches!(state.events[events + k], GameEvent::Tapped { object } if object == ObjectId(i)));
    }
    assert!(asked_each(zero.len(), |k| Act::Move(ObjectId(zero[k]), Zone::Graveyard)));
    assert!(state.creature_died_this_turn == (died || !zero.is_empty()));
    let mut expected = [None; 3];
    let mut n = 0;
    for &i in destroyed {
        let id = ObjectId(i);
        if has_keyword(&state, id, Keyword::Indestructible, &registry) {
            continue;
        }
        expected[n] = Some((shielded[index(id)], id));
        n += 1;
    }
    // SAFETY: as in `is_creature`.
    let (list, m) = unsafe { (&*std::ptr::addr_of!(DONE), *std::ptr::addr_of!(DONE_N)) };
    assert!(m == n && (0..n).all(|k| list[k] == expected[k]));
    assert!(acted == (!zero.is_empty() || n > 0));
    std::mem::forget((registry, state));
}

/// The proofs of removing, the destruction and regeneration recorded.
macro_rules! each_removal {
    ($($name:ident: $zero:expr, $destroyed:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::destruction::regenerate_sba, regenerate_sba)]
            #[kani::stub(crate::destruction::destroy_sba, destroy_sba)]
            #[kani::stub(crate::destruction::death_event, death_event)]
            #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
            /// [`removing`] for the creatures of its name.
            fn $name() {
                removing(&$zero, &$destroyed);
            }
        }
    )*};
}

each_removal! {
    removing_none_does_nothing: [], [];
    a_creature_of_no_toughness_goes_to_the_graveyard_and_a_destroyed_one_is_destroyed: [1], [2];
    two_destroyed_creatures_are_each_destroyed_regenerated_or_left: [], [1, 2];
    two_creatures_of_no_toughness_go_to_the_graveyard_in_order: [2, 1], [];
}

/// The control effects that end with their condition (`GameState::
/// expire_control_effects`, CR 611.2b, 613.7a): each effect "for as long
/// as" its player controls its source ends once the source is not on the
/// battlefield under that player, and the others stay, in their order;
/// then each that ended, in order, gives its permanent, if on the
/// battlefield, to whom the effects still in force give it (the latest of
/// them on it), or with none to its own controller. Whether any ended. For
/// four objects (sources 1 and 4, permanents 2 and 3), each on the
/// battlefield or in a graveyard, under either player and with either as
/// its own controller, and `effects` a constant of each harness: each
/// effect's permanent and source, each effect giving its permanent to
/// either player for as long as either controls the source, each later
/// than the one before.
fn expiring(effects: &[(u64, u64)]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let (mut on, mut controller, mut base) = ([false; 4], [PlayerId(0); 4], [PlayerId(0); 4]);
    for i in 0..4 {
        on[i] = kani::any();
        controller[i] = PlayerId(u8::from(kani::any::<bool>()));
        base[i] = PlayerId(u8::from(kani::any::<bool>()));
        let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
        let id = state.create_object(blank, controller[i], zone, Some(1), Some(1));
        state.get_object_mut(id).expect("just created").base_controller = base[i];
    }
    let mut given = [(PlayerId(0), PlayerId(0)); 2];
    state.control_effects = Vec::with_capacity(effects.len());
    for (k, &(object, source)) in effects.iter().enumerate() {
        given[k] = (PlayerId(u8::from(kani::any::<bool>())), PlayerId(u8::from(kani::any::<bool>())));
        state.control_effects.push(crate::state::ControlEffect {
            object: ObjectId(object),
            controller: given[k].0,
            original_controller: base[(object - 1) as usize],
            source: ObjectId(source),
            source_controller: given[k].1,
            timestamp: 10 + k as u64,
        });
    }

    let did = state.expire_control_effects();

    let at = |id: u64| (id - 1) as usize;
    let ends = |k: usize| {
        let s = at(effects[k].1);
        !(on[s] && controller[s] == given[k].1)
    };
    // The effects left, in order.
    let mut left = 0;
    for (k, &(object, _)) in effects.iter().enumerate() {
        if !ends(k) {
            let e = &state.control_effects[left];
            assert!(e.object == ObjectId(object) && e.controller == given[k].0 && e.timestamp == 10 + k as u64);
            left += 1;
        }
    }
    assert!(state.control_effects.len() == left);
    // Who each permanent goes to: the latest effect left on it, or its own
    // controller.
    let goes_to = |object: u64| {
        let mut to = base[at(object)];
        for (k, &(o, _)) in effects.iter().enumerate() {
            if o == object && !ends(k) {
                to = given[k].0;
            }
        }
        to
    };
    let mut back = [(ObjectId(0), PlayerId(0)); 2];
    let mut n = 0;
    for (k, &(object, _)) in effects.iter().enumerate() {
        if ends(k) && on[at(object)] {
            back[n] = (ObjectId(object), goes_to(object));
            n += 1;
        }
    }
    assert!(asked_each(n, |i| Act::Control(back[i].0, back[i].1)));
    assert!(did == (0..effects.len()).any(ends));
    std::mem::forget((registry, state));
}

/// The proofs of control effects ending, a change of control recorded.
macro_rules! each_expiry {
    ($($name:ident: $effects:expr;)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::state::GameState::change_control, crate::proofs::cards::record_control)]
        /// [`expiring`] for the effects of its name.
        fn $name() {
            expiring(&$effects);
        }
    )*};
}

each_expiry! {
    a_control_effect_ends_when_its_source_leaves_or_changes_hands: [(2, 1)];
    two_control_effects_on_one_permanent_from_two_sources_end_each_with_its_own: [(2, 1), (2, 4)];
    control_effects_on_two_permanents_from_one_source_end_together: [(2, 1), (3, 1)];
    a_later_control_effect_from_the_same_source_ends_with_the_earlier: [(2, 1), (2, 1)];
}

/// A player loses for their life or their draws (`players_lose`, CR
/// 704.5a-b, 104.3): each player still in the game with 0 or less life
/// loses, for that; each other one still in it who drew from an empty
/// library loses, for that; each loss is announced once, those for life
/// first, in the players' order, then those for drawing; and a player out
/// of the game already is left as they were. Whether anyone lost. For each
/// player out of the game or not, of life -1 to 1, having drawn from an
/// empty library or not; the loss itself is `player_loses`, run as it is
/// (`simple_actions` proves it).
#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
fn a_player_of_no_life_or_who_drew_from_an_empty_library_loses() {
    let mut state = crate::proofs::cards::new_game();
    let mut was = [(false, 0i32, false); 2];
    for (i, w) in was.iter_mut().enumerate() {
        let (lost, life, drew): (bool, i32, bool) = (kani::any(), kani::any(), kani::any());
        kani::assume((-1..=1).contains(&life));
        *w = (lost, life, drew);
        let p = &mut state.players[i];
        p.lost = lost;
        p.loss_reason = lost.then_some(LossReason::Conceded);
        p.life = life;
        p.has_drawn_from_empty = drew;
    }
    let events = state.events.len();

    let did = crate::sba::players_lose(&mut state);

    let reason = |(lost, life, drew): (bool, i32, bool)| {
        if lost {
            None
        } else if life <= 0 {
            Some(LossReason::LifeReachedZero)
        } else {
            drew.then_some(LossReason::DrewFromEmptyLibrary)
        }
    };
    let mut announced = [None; 2];
    let mut n = 0;
    for r in [LossReason::LifeReachedZero, LossReason::DrewFromEmptyLibrary] {
        for (i, &w) in was.iter().enumerate() {
            if reason(w) == Some(r) {
                announced[n] = Some((PlayerId(i as u8), r));
                n += 1;
            }
        }
    }
    for (i, &w) in was.iter().enumerate() {
        let p = &state.players[i];
        assert!(p.id == PlayerId(i as u8));
        match reason(w) {
            Some(r) => { assert!(p.lost && p.loss_reason == Some(r)); }
            None => { assert!(p.lost == w.0 && p.loss_reason == w.0.then_some(LossReason::Conceded)); }
        }
    }
    assert!(state.events.len() == events + n);
    for (k, &a) in announced.iter().enumerate().take(n) {
        assert!(matches!(state.events[events + k], GameEvent::PlayerLost { player, reason } if Some((player, reason)) == a));
    }
    assert!(did == (n > 0));
    std::mem::forget(state);
}

/// Whether each of the board's objects is attached illegally, and whether
/// it is an Equipment: any answer.
static mut ILLEGAL: [Option<bool>; 3] = [None; 3];
static mut EQUIPMENT: [Option<bool>; 3] = [None; 3];

/// Stands in for `attachment::illegality`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn illegality(_: &GameState, id: ObjectId, _: &CardRegistry) -> Option<crate::attachment::Illegality> {
    // SAFETY: as in `is_creature`, for ILLEGAL.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ILLEGAL) };
    (*answers[index(id)].get_or_insert_with(kani::any::<bool>)).then_some(crate::attachment::Illegality::HostGone)
}

/// Stands in for `GameState::is_equipment`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn is_equipment(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: as in `is_creature`, for EQUIPMENT.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(EQUIPMENT) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::unattach` (what it does is
/// [`an_attachment_comes_off_the_permanent_or_player_it_is_on`]'s):
/// recorded, as off a permanent.
#[allow(dead_code)] // named only by `kani::stub`
fn unattach(_: &mut GameState, id: ObjectId) -> Option<ObjectId> {
    done(false, id);
    None
}

/// Stands in for `GameState::unattach_from_player`: recorded, as off a
/// player.
#[allow(dead_code)] // named only by `kani::stub`
fn unattach_from_player(_: &mut GameState, id: ObjectId) -> Option<PlayerId> {
    done(true, id);
    None
}

card_proof! {
    /// An attachment comes off what it is on (`unattach`,
    /// `unattach_from_player`, CR 701.3d, 704.5n): off a permanent, it is
    /// attached to none, the permanent it was on is answered, and whether
    /// it was on a player is left as it was; off a player, the same of the
    /// player. With nothing to come off, nothing is answered. For an
    /// object on a permanent or not and on either player or neither.
    fn an_attachment_comes_off_the_permanent_or_player_it_is_on() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let host = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
        let id = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
        let on_host = kani::any::<bool>().then_some(host);
        let on_player = kani::any::<bool>().then(|| PlayerId(u8::from(kani::any::<bool>())));
        let o = state.get_object_mut(id).expect("just created");
        (o.attached_to, o.attached_to_player) = (on_host, on_player);

        if kani::any() {
            assert!(state.unattach_from_player(id) == on_player);
            let o = state.get_object(id).expect("on the board");
            assert!(o.attached_to_player.is_none() && o.attached_to == on_host);
        } else {
            assert!(state.unattach(id) == on_host);
            let o = state.get_object(id).expect("on the board");
            assert!(o.attached_to.is_none() && o.attached_to_player == on_player);
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    #[kani::stub(crate::attachment::illegality, illegality)]
    #[kani::stub(crate::state::GameState::is_equipment, is_equipment)]
    #[kani::stub(crate::state::GameState::unattach, unattach)]
    #[kani::stub(crate::state::GameState::unattach_from_player, unattach_from_player)]
    /// What the pass does to the permanents attached illegally
    /// (`remove_illegal_attachments`, CR 704.5m-n, 301.5c): each Equipment
    /// of them becomes unattached, in the board's order, from the
    /// permanent it is on or else from the player, and stays on the
    /// battlefield; then each other (an Aura) goes to its owner's
    /// graveyard, in the board's order. Whether any did. For three
    /// objects, each on the battlefield or in a graveyard, attached to a
    /// permanent or not, attached illegally or not and an Equipment or not
    /// (each any answer: which attachments are illegal is
    /// `attachment::illegality`'s).
    fn an_illegal_equipment_comes_off_and_an_illegal_aura_goes_to_the_graveyard() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let (mut on, mut attached) = ([false; 3], [false; 3]);
        for i in 0..3 {
            on[i] = kani::any();
            let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
            let id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), zone, None, None);
            attached[i] = kani::any();
            state.get_object_mut(id).expect("just created").attached_to = attached[i].then_some(ObjectId(1));
        }

        let did = crate::sba::remove_illegal_attachments(&mut state, &registry);

        let (mut equipment, mut auras) = ([None; 3], [ObjectId(0); 3]);
        let (mut ne, mut na) = (0, 0);
        for i in 0..3 {
            let id = ObjectId(i as u64 + 1);
            if !on[i] || illegality(&state, id, &registry).is_none() {
                continue;
            }
            if is_equipment(&state, id, &registry) {
                equipment[ne] = Some((!attached[i], id));
                ne += 1;
            } else {
                auras[na] = id;
                na += 1;
            }
        }
        // SAFETY: as in `is_creature`.
        let (list, m) = unsafe { (&*std::ptr::addr_of!(DONE), *std::ptr::addr_of!(DONE_N)) };
        assert!(m == ne && (0..ne).all(|k| list[k] == equipment[k]));
        assert!(asked_each(na, |k| Act::Move(auras[k], Zone::Graveyard)));
        assert!(did == (ne + na > 0));
        std::mem::forget((registry, state));
    }
}

/// Whether each object is an Aura, an Equipment and a creature, and
/// whether the host has protection from the attachment and the player may
/// be enchanted by it, and whether the host passes the enchant's filter:
/// any answer.
static mut AURA: [Option<bool>; 3] = [None; 3];
static mut EQUIPPING: [Option<bool>; 3] = [None; 3];
static mut CREATURE_HOST: [Option<bool>; 3] = [None; 3];
static mut PROTECTED: Option<bool> = None;
static mut ENCHANTABLE: Option<bool> = None;
static mut FILTERED: Option<bool> = None;

fn once(answer: *mut Option<bool>) -> bool {
    // SAFETY: a proof runs on one thread, and each answer is named only
    // through here.
    *unsafe { &mut *answer }.get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::has_subtype`, asked only "Aura" and
/// "Equipment": any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn has_subtype(_: &GameState, id: ObjectId, subtype: &str, _: &CardRegistry) -> bool {
    // SAFETY: as in `is_creature`, for AURA and EQUIPPING.
    let answers = unsafe {
        match subtype {
            "Aura" => &mut *std::ptr::addr_of_mut!(AURA),
            "Equipment" => &mut *std::ptr::addr_of_mut!(EQUIPPING),
            _ => panic!("a subtype the question does not ask"),
        }
    };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::is_creature`, of the host: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn host_is_creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: as in `is_creature`, for CREATURE_HOST.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(CREATURE_HOST) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_protection_from(_: &GameState, target: ObjectId, source: ObjectId, _: &CardRegistry) -> bool {
    assert!(target == ObjectId(2) && source == ObjectId(1), "a protection the question does not ask");
    once(std::ptr::addr_of_mut!(PROTECTED))
}

#[allow(dead_code)] // named only by `kani::stub`
fn player_can_be_enchanted_by(_: &GameState, aura: ObjectId, _: PlayerId, _: &CardRegistry) -> bool {
    assert!(aura == ObjectId(1), "an Aura the question does not ask about");
    once(std::ptr::addr_of_mut!(ENCHANTABLE))
}

#[allow(dead_code)] // named only by `kani::stub`
fn matches_target_filter(
    _: &GameState, host: &crate::state::GameObject, _: &crate::cards::TargetFilter, _: PlayerId, source: Option<ObjectId>,
    _: &CardRegistry,
) -> bool {
    assert!(host.id == ObjectId(2) && source == Some(ObjectId(1)), "a filter the question does not ask");
    once(std::ptr::addr_of_mut!(FILTERED))
}

/// An attachment whose enchant ability is `.0` (for an Aura).
struct Enchanting(TargetRequirement);

impl CardBehavior for Enchanting {
    fn card_data(&self) -> CardData {
        CardData { name: "Enchanting".into(), ..CardData::default() }
    }
    fn target_requirement(&self) -> TargetRequirement {
        self.0.clone()
    }
}

/// Whether an attachment is attached illegally, and why
/// (`attachment::illegality`, CR 704.5m-n, 301.5, 303.4, 702.5d, 702.16c-d):
/// only an Aura or an Equipment on the battlefield can be. Attached to a
/// permanent, it is if that permanent is not on the battlefield (gone),
/// else if it is not one the attachment may be on (an Equipment's must be
/// a creature; an Aura's what its enchant names: a creature, a creature
/// its filter passes, no permanent for one that enchants a player, and
/// anything for an enchant no requirement says), else if it has
/// protection from the attachment, in that order. Attached to a player, it
/// is if it is an Equipment, or the player is not in the game's list, or
/// it does not enchant a player, or the player cannot be enchanted by it.
/// Attached to nothing, an Aura is and an Equipment is not. For the
/// attachment on the battlefield or in a graveyard, an Aura or not and an
/// Equipment or not; attached to a permanent on the battlefield, one in a
/// graveyard, one not on the board, or none; to either player, one not in
/// the game, or none; and each other question any answer.
fn judging(enchant: TargetRequirement) {
    use crate::attachment::Illegality;
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Enchanting(enchant.clone())));
    let mut state = crate::proofs::cards::new_game();
    let on: bool = kani::any();
    let id = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let host_on: bool = kani::any();
    let host = state.create_object(blank, PlayerId(0), if host_on { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
    let to_object = match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some(host),
        _ => Some(ObjectId(9)),
    };
    let to_player = match kani::any::<u8>() % 4 {
        0 => None,
        1 => Some(PlayerId(0)),
        2 => Some(PlayerId(1)),
        _ => Some(PlayerId(5)),
    };
    let o = state.get_object_mut(id).expect("just created");
    (o.attached_to, o.attached_to_player) = (to_object, to_player);

    let judged = crate::attachment::illegality(&state, id, &registry);

    let aura = has_subtype(&state, id, "Aura", &registry);
    let equipment = has_subtype(&state, id, "Equipment", &registry);
    let enchants_players = matches!(enchant, TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly);
    let expected = if !on || !(aura || equipment) {
        None
    } else if let Some(h) = to_object {
        if h != host || !host_on {
            Some(Illegality::HostGone)
        } else if !(if equipment {
            host_is_creature(&state, host, &registry)
        } else {
            match enchant {
                TargetRequirement::PlayerOnly | TargetRequirement::OpponentOnly => false,
                TargetRequirement::Creature => host_is_creature(&state, host, &registry),
                TargetRequirement::CreatureWithFilter(_) => {
                    host_is_creature(&state, host, &registry) && once(std::ptr::addr_of_mut!(FILTERED))
                }
                _ => true,
            }
        }) {
            Some(Illegality::HostIneligible)
        } else if once(std::ptr::addr_of_mut!(PROTECTED)) {
            Some(Illegality::HostProtected)
        } else {
            None
        }
    } else if let Some(p) = to_player {
        (equipment || p.0 > 1 || !enchants_players || !once(std::ptr::addr_of_mut!(ENCHANTABLE))).then_some(Illegality::PlayerIneligible)
    } else {
        aura.then_some(Illegality::Unattached)
    };
    assert!(judged == expected);
    std::mem::forget((registry, state, enchant));
}

/// The proofs of judging an attachment, one per enchant.
macro_rules! each_enchant {
    ($($name:ident: $enchant:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::has_subtype, has_subtype)]
            #[kani::stub(crate::state::GameState::is_creature, host_is_creature)]
            #[kani::stub(crate::state::GameState::has_protection_from, has_protection_from)]
            #[kani::stub(crate::state::GameState::player_can_be_enchanted_by, player_can_be_enchanted_by)]
            #[kani::stub(crate::engine::targeting::matches_target_filter, matches_target_filter)]
            /// [`judging`] for the enchant of its name.
            fn $name() {
                judging($enchant);
            }
        }
    )*};
}

each_enchant! {
    an_attachment_enchanting_a_creature_is_judged_as_the_rules_say: TargetRequirement::Creature;
    an_attachment_enchanting_a_filtered_creature_is_judged_as_the_rules_say:
        TargetRequirement::CreatureWithFilter(crate::cards::TargetFilter::YouControl);
    an_attachment_enchanting_a_player_is_judged_as_the_rules_say: TargetRequirement::PlayerOnly;
    an_attachment_enchanting_an_opponent_is_judged_as_the_rules_say: TargetRequirement::OpponentOnly;
    an_attachment_of_an_enchant_no_requirement_says_is_judged_as_the_rules_say: TargetRequirement::None;
}

card_proof! {
    /// +1/+1 and -1/-1 counters annihilate (`annihilate_counters`, CR
    /// 704.5q): a permanent with some of each loses as many of each as the
    /// fewer of them, and every other object's are left as they were.
    /// Whether any did. For three objects, each on the battlefield or in a
    /// graveyard, with 0 to 2 of each.
    fn plus_and_minus_counters_on_a_permanent_annihilate_in_pairs() {
        use crate::types::CounterType::{MinusOneMinusOne, PlusOnePlusOne};
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let (mut on, mut plus, mut minus) = ([false; 3], [0u32; 3], [0u32; 3]);
        for i in 0..3 {
            on[i] = kani::any();
            let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
            let id = state.create_object(blank, PlayerId(0), zone, Some(1), Some(1));
            (plus[i], minus[i]) = (kani::any(), kani::any());
            kani::assume(plus[i] <= 2 && minus[i] <= 2);
            let o = state.get_object_mut(id).expect("just created");
            o.counters.insert(PlusOnePlusOne, plus[i]);
            o.counters.insert(MinusOneMinusOne, minus[i]);
        }

        let did = crate::sba::annihilate_counters(&mut state);

        let mut any = false;
        for i in 0..3 {
            let o = state.get_object(ObjectId(i as u64 + 1)).expect("on the board");
            let (p, m) = (o.counters.get(&PlusOnePlusOne).copied().unwrap_or(0), o.counters.get(&MinusOneMinusOne).copied().unwrap_or(0));
            if on[i] && plus[i] > 0 && minus[i] > 0 {
                let k = plus[i].min(minus[i]);
                assert!(p == plus[i] - k && m == minus[i] - k);
                any = true;
            } else {
                assert!(p == plus[i] && m == minus[i]);
            }
        }
        assert!(did == any);
        std::mem::forget((registry, state));
    }
}

/// Whether each object's state trigger condition holds: any answer.
static mut CONDITION: [Option<bool>; 3] = [None; 3];

/// A permanent with a state-triggered ability (CR 603.8) whose condition
/// is any answer.
struct StateTriggered;

impl CardBehavior for StateTriggered {
    fn card_data(&self) -> CardData {
        CardData { name: "StateTriggered".into(), ..CardData::default() }
    }
    fn state_trigger_condition(&self, _: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
        // SAFETY: as in `is_creature`, for CONDITION.
        let answers = unsafe { &mut *std::ptr::addr_of_mut!(CONDITION) };
        *answers[index(id)].get_or_insert_with(kani::any::<bool>)
    }
    fn state_trigger_description(&self) -> String {
        "When".into()
    }
}

card_proof! {
    /// A state-triggered ability triggers (`raise_state_trigger`, CR
    /// 603.8): of the permanents on the battlefield whose ability is not
    /// on the stack already, the first in the board's order whose
    /// condition holds is marked as having it on the stack, and its
    /// ability is the one trigger waiting, from it, under its controller,
    /// with its card's description, of no target and not yet ordered; the
    /// pass stops there. With none, nothing is done. For three such
    /// permanents, each either player's, on the battlefield or in a
    /// graveyard, its ability on the stack or not, and its condition any
    /// answer.
    fn the_first_permanent_whose_state_trigger_condition_holds_triggers() {
        let mut registry = CardRegistry::new();
        let card = registry.register(Box::new(StateTriggered));
        let mut state = crate::proofs::cards::new_game();
        let (mut on, mut flagged, mut controller) = ([false; 3], [false; 3], [PlayerId(0); 3]);
        for i in 0..3 {
            on[i] = kani::any();
            controller[i] = PlayerId(u8::from(kani::any::<bool>()));
            let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
            let id = state.create_object(card, controller[i], zone, None, None);
            flagged[i] = kani::any();
            state.get_object_mut(id).expect("just created").state_trigger_on_stack = flagged[i];
        }

        let did = crate::sba::raise_state_trigger(&mut state, &registry);

        let mut first = None;
        for i in 0..3 {
            if on[i] && !flagged[i] && StateTriggered.state_trigger_condition(&state, ObjectId(i as u64 + 1), &registry) {
                first = Some(i);
                break;
            }
        }
        for i in 0..3 {
            let o = state.get_object(ObjectId(i as u64 + 1)).expect("on the board");
            assert!(o.state_trigger_on_stack == (flagged[i] || first == Some(i)));
        }
        match first {
            Some(i) => {
                assert!(did && state.pending_triggers.len() == 1);
                let t = &state.pending_triggers[0];
                assert!(t.source.id == ObjectId(i as u64 + 1) && t.source.card_id == card && t.source.controller == controller[i]);
                assert!(t.source.description == "When" && t.source.chosen_targets.is_empty() && !t.ordered);
                assert!(matches!(t.event, crate::triggers::TriggerEvent::StateTriggered));
            }
            None => { assert!(!did && state.pending_triggers.is_empty()); }
        }
        std::mem::forget((registry, state));
    }
}

/// Whether each object is a planeswalker: any answer.
static mut WALKER: [Option<bool>; 3] = [None; 3];

/// Stands in for `GameState::has_card_type`, asked only whether an object
/// is a planeswalker: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn has_card_type(_: &GameState, id: ObjectId, card_type: CardType, _: &CardRegistry) -> bool {
    assert!(card_type == CardType::Planeswalker, "a type the pass does not ask");
    // SAFETY: as in `is_creature`, for WALKER.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(WALKER) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

card_proof! {
    #[kani::stub(crate::state::GameState::has_card_type, has_card_type)]
    /// A planeswalker with no loyalty goes to the graveyard
    /// (`remove_planeswalkers_without_loyalty`, CR 704.5i): each permanent
    /// that is a planeswalker with no loyalty counters (none at all, or
    /// none left) goes to its owner's graveyard, in the board's order.
    /// Whether any did. For three objects, each on the battlefield or in a
    /// graveyard, with no loyalty counters kept, or 0 or 1, a planeswalker
    /// or not (any answer).
    fn a_planeswalker_with_no_loyalty_goes_to_the_graveyard() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let (mut on, mut loyalty) = ([false; 3], [0u32; 3]);
        for i in 0..3 {
            on[i] = kani::any();
            let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
            let id = state.create_object(blank, PlayerId(0), zone, None, None);
            if kani::any() {
                loyalty[i] = u32::from(kani::any::<bool>());
                state.get_object_mut(id).expect("just created").counters.insert(CounterType::Loyalty, loyalty[i]);
            }
        }

        let did = crate::sba::remove_planeswalkers_without_loyalty(&mut state, &registry);

        let mut gone = [ObjectId(0); 3];
        let mut n = 0;
        for i in 0..3 {
            let id = ObjectId(i as u64 + 1);
            if on[i] && loyalty[i] == 0 && has_card_type(&state, id, CardType::Planeswalker, &registry) {
                gone[n] = id;
                n += 1;
            }
        }
        assert!(asked_each(n, |k| Act::Move(gone[k], Zone::Graveyard)));
        assert!(did == (n > 0));
        std::mem::forget((registry, state));
    }
}

/// Whether each object is legendary, and whether it is named "B" (else
/// "A"): any answer.
static mut LEGENDARY: [Option<bool>; 3] = [None; 3];
static mut NAMED_B: [Option<bool>; 3] = [None; 3];

/// Stands in for `GameState::is_legendary`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn is_legendary(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: as in `is_creature`, for LEGENDARY.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(LEGENDARY) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

fn named_b(id: ObjectId) -> bool {
    // SAFETY: as in `is_creature`, for NAMED_B.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(NAMED_B) };
    *answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `GameState::name_of`: "A" or "B", any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn name_of(_: &GameState, id: ObjectId, _: &CardRegistry) -> String {
    if named_b(id) { "B" } else { "A" }.into()
}

/// The legend rule as the pass applies it (`apply_legend_rule`, CR
/// 704.5j): of the groups of legendary permanents one player controls
/// that share a name, the first, by player and then name, of two or more
/// has its player asked which one to keep (not optionally), each of the
/// group a choice, in the board's order, and the pass stops there; with
/// no such group nothing is asked. While another question is waiting,
/// nothing is asked and no name is read. For three objects, each either
/// player's, on the battlefield or in a graveyard, legendary or not, and
/// named "A" or "B".
fn legends(waiting: bool) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let (mut on, mut controller) = ([false; 3], [PlayerId(0); 3]);
    for i in 0..3 {
        on[i] = kani::any();
        controller[i] = PlayerId(u8::from(kani::any::<bool>()));
        let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
        state.create_object(blank, controller[i], zone, None, None);
    }
    if waiting {
        state.awaiting_action = Some(crate::state::AwaitingAction::DeclareAttackers);
    }

    let did = crate::sba::apply_legend_rule(&mut state, &registry);

    if waiting {
        // SAFETY: as in `is_creature`.
        let (legendary, named) = unsafe { (&*std::ptr::addr_of!(LEGENDARY), &*std::ptr::addr_of!(NAMED_B)) };
        assert!(!did && acted() == 0 && legendary.iter().chain(named).all(Option::is_none));
        std::mem::forget((registry, state));
        return;
    }
    let mut asked = None;
    'groups: for p in [PlayerId(0), PlayerId(1)] {
        for b in [false, true] {
            let (mut ids, mut n) = ([ObjectId(0); 3], 0);
            for i in 0..3 {
                let id = ObjectId(i as u64 + 1);
                if on[i] && controller[i] == p && named_b(id) == b && is_legendary(&state, id, &registry) {
                    ids[n] = id;
                    n += 1;
                }
            }
            if n > 1 {
                asked = Some((p, b, ids, n));
                break 'groups;
            }
        }
    }
    match asked {
        Some((player, b, ids, n)) => {
            assert!(did && asked_each(1, |_| Act::Choose {
                source: ids[0],
                player,
                options: ids[..n].iter().map(|&id| Target::Object(id)).collect(),
                effect: PendingEffect::LegendRuleKeep { player, legend_name: if b { "B" } else { "A" }.into() },
                optional: false,
            }));
        }
        None => { assert!(!did && acted() == 0); }
    }
    std::mem::forget((registry, state));
}

card_proof! {
    #[kani::stub(crate::state::GameState::is_legendary, is_legendary)]
    #[kani::stub(crate::state::GameState::name_of, name_of)]
    /// [`legends`], no other question waiting.
    fn a_player_with_two_legends_of_one_name_is_asked_which_to_keep() {
        legends(false);
    }
}

card_proof! {
    #[kani::stub(crate::state::GameState::is_legendary, is_legendary)]
    #[kani::stub(crate::state::GameState::name_of, name_of)]
    /// [`legends`], another question waiting.
    fn the_legend_rule_waits_while_another_question_is_waiting() {
        legends(true);
    }
}

/// The objects that ceased to exist, in order.
static mut CEASED: [ObjectId; 3] = [ObjectId(0); 3];
static mut CEASED_N: usize = 0;

/// Stands in for `GameState::cease_to_exist` (its own proof below):
/// recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn cease_to_exist(_: &mut GameState, id: ObjectId) {
    // SAFETY: as in `is_creature`, for CEASED and CEASED_N.
    let (list, n) = unsafe { (&mut *std::ptr::addr_of_mut!(CEASED), &mut *std::ptr::addr_of_mut!(CEASED_N)) };
    list[*n] = id;
    *n += 1;
}

card_proof! {
    #[kani::stub(crate::state::GameState::cease_to_exist, cease_to_exist)]
    /// A token anywhere but the battlefield ceases to exist
    /// (`remove_tokens_elsewhere`, CR 704.5d, 111.7): each, in the board's
    /// order, and nothing else. Whether any did. For three objects, each a
    /// token or not, on the battlefield, in a graveyard or in exile.
    fn a_token_off_the_battlefield_ceases_to_exist() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let mut gone = [ObjectId(0); 3];
        let mut n = 0;
        for _ in 0..3 {
            let zone = match kani::any::<u8>() % 3 {
                0 => Zone::Battlefield,
                1 => Zone::Graveyard,
                _ => Zone::Exile,
            };
            let id = state.create_object(blank, PlayerId(0), zone, None, None);
            let token: bool = kani::any();
            state.get_object_mut(id).expect("just created").is_token = token;
            if token && zone != Zone::Battlefield {
                gone[n] = id;
                n += 1;
            }
        }

        let did = crate::sba::remove_tokens_elsewhere(&mut state);

        // SAFETY: as in `is_creature`.
        let (list, m) = unsafe { (&*std::ptr::addr_of!(CEASED), *std::ptr::addr_of!(CEASED_N)) };
        assert!(m == n && (0..n).all(|k| list[k] == gone[k]));
        assert!(did == (n > 0));
        std::mem::forget((registry, state));
    }
}

/// An object ceases to exist (`cease_to_exist`, 704.5d): it is no longer
/// on the board, and no library lists it, each library's other cards kept
/// in their order; every other object is kept. For each of three objects
/// (`id` a constant of each harness: removed from the board at an id the
/// solver leaves open, it ran symbolic execution out of memory), each
/// library listing some of them.
fn ceasing(id: u64) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    for _ in 0..3 {
        state.create_object(blank, PlayerId(0), Zone::Library, None, None);
    }
    state.players[0].library_order = vec![ObjectId(1), ObjectId(2), ObjectId(3)];
    state.players[1].library_order = vec![ObjectId(3), ObjectId(1)];

    state.cease_to_exist(ObjectId(id));

    for i in 1..=3 {
        assert!(state.get_object(ObjectId(i)).is_some() == (i != id));
    }
    let kept = |list: &[ObjectId], order: &[u64]| {
        let mut k = 0;
        for &i in order {
            if i != id {
                if list.get(k) != Some(&ObjectId(i)) {
                    return false;
                }
                k += 1;
            }
        }
        list.len() == k
    };
    assert!(kept(&state.players[0].library_order, &[1, 2, 3]) && kept(&state.players[1].library_order, &[3, 1]));
    std::mem::forget((registry, state));
}

card_proof! {
    /// [`ceasing`], of the first object.
    fn the_first_object_ceasing_to_exist_is_gone_from_the_board_and_every_library() {
        ceasing(1);
    }
}

card_proof! {
    /// [`ceasing`], of the second object.
    fn the_second_object_ceasing_to_exist_is_gone_from_the_board_and_every_library() {
        ceasing(2);
    }
}

card_proof! {
    /// [`ceasing`], of the third object.
    fn the_third_object_ceasing_to_exist_is_gone_from_the_board_and_every_library() {
        ceasing(3);
    }
}

card_proof! {
    /// The game ends when it is decided (`end_game_if_decided`, CR 104.2a,
    /// 104.4a): of a game in progress, with one player left in it that
    /// player wins, with none it is a draw, and with both nothing is done;
    /// a game over already is left as it is. Whether it ended. For each
    /// player out of the game or not, and a game in progress or a draw
    /// already.
    fn the_game_ends_with_one_player_left_or_none() {
        let mut state = crate::proofs::cards::new_game();
        let lost: [bool; 2] = [kani::any(), kani::any()];
        state.players[0].lost = lost[0];
        state.players[1].lost = lost[1];
        let over: bool = kani::any();
        if over {
            state.result = Some(crate::state::GameResult::Draw);
        }

        let did = state.end_game_if_decided();

        let result = match (over, lost) {
            (true, _) | (false, [false, false]) => None,
            (false, [true, true]) => Some(crate::state::GameResult::Draw),
            (false, [false, true]) => Some(crate::state::GameResult::Winner(PlayerId(0))),
            (false, [true, false]) => Some(crate::state::GameResult::Winner(PlayerId(1))),
        };
        match result {
            Some(r) => { assert!(did && asked_for(&[Act::GameEnds(r)])); }
            None => { assert!(!did && acted() == 0); }
        }
        std::mem::forget(state);
    }
}

/// The parts of a pass called, in order, by number (as in [`part`]), and
/// what each answered.
static mut CALLED: [u8; 11] = [u8::MAX; 11];
static mut CALLED_N: usize = 0;
static mut ANSWERED: [bool; 11] = [false; 11];
/// Whether the creatures removed were the ones decided.
static mut HANDED_ON: bool = false;

/// A part of a pass called: recorded, and any answer.
fn part(n: u8) -> bool {
    // SAFETY: as in `is_creature`, for CALLED, CALLED_N and ANSWERED.
    let (called, k, answered) = unsafe {
        (&mut *std::ptr::addr_of_mut!(CALLED), &mut *std::ptr::addr_of_mut!(CALLED_N), &mut *std::ptr::addr_of_mut!(ANSWERED))
    };
    called[*k] = n;
    *k += 1;
    answered[usize::from(n)] = kani::any();
    answered[usize::from(n)]
}

#[allow(dead_code)] // each named only by `kani::stub`
fn part_expire(_: &mut GameState) -> bool {
    part(0)
}
#[allow(dead_code)]
fn part_lose(_: &mut GameState) -> bool {
    part(1)
}
#[allow(dead_code)]
fn part_fates(_: &GameState, _: &CardRegistry) -> (Vec<ObjectId>, Vec<ObjectId>) {
    part(2);
    (vec![ObjectId(1)], vec![ObjectId(2), ObjectId(3)])
}
#[allow(dead_code)]
fn part_remove(_: &mut GameState, zero: Vec<ObjectId>, destroyed: Vec<ObjectId>, _: &CardRegistry) -> bool {
    // SAFETY: as in `is_creature`, for HANDED_ON.
    unsafe { *std::ptr::addr_of_mut!(HANDED_ON) = zero == [ObjectId(1)] && destroyed == [ObjectId(2), ObjectId(3)] };
    std::mem::forget((zero, destroyed));
    part(3)
}
#[allow(dead_code)]
fn part_attachments(_: &mut GameState, _: &CardRegistry) -> bool {
    part(4)
}
#[allow(dead_code)]
fn part_counters(_: &mut GameState) -> bool {
    part(5)
}
#[allow(dead_code)]
fn part_state_trigger(_: &mut GameState, _: &CardRegistry) -> bool {
    part(6)
}
#[allow(dead_code)]
fn part_walkers(_: &mut GameState, _: &CardRegistry) -> bool {
    part(7)
}
#[allow(dead_code)]
fn part_legends(_: &mut GameState, _: &CardRegistry) -> bool {
    part(8)
}
#[allow(dead_code)]
fn part_tokens(_: &mut GameState) -> bool {
    part(9)
}
#[allow(dead_code)]
fn part_game_end(_: &mut GameState) -> bool {
    part(10)
}

// Walks over the eleven parts, unwound past them.
card_proof! {
    unwind(12)
    #[kani::stub(crate::state::GameState::expire_control_effects, part_expire)]
    #[kani::stub(crate::sba::players_lose, part_lose)]
    #[kani::stub(crate::sba::creature_fates, part_fates)]
    #[kani::stub(crate::sba::remove_creatures, part_remove)]
    #[kani::stub(crate::sba::remove_illegal_attachments, part_attachments)]
    #[kani::stub(crate::sba::annihilate_counters, part_counters)]
    #[kani::stub(crate::sba::raise_state_trigger, part_state_trigger)]
    #[kani::stub(crate::sba::remove_planeswalkers_without_loyalty, part_walkers)]
    #[kani::stub(crate::sba::apply_legend_rule, part_legends)]
    #[kani::stub(crate::sba::remove_tokens_elsewhere, part_tokens)]
    #[kani::stub(crate::state::GameState::end_game_if_decided, part_game_end)]
    /// One pass (`one_pass`, CR 704.3, 704.5, 603.8): the parts, each
    /// proved above, are taken once each, in order: the control effects
    /// that end (611.2b), the players' losses, the creatures' fates and
    /// their removal (the creatures removed exactly those decided), the
    /// illegal attachments, the counters, the state triggers; if one
    /// triggered, the pass stops there; then the planeswalkers, the legend
    /// rule, and if a player is asked, the pass stops there; then the
    /// tokens, and the game's end. Otherwise the pass acted exactly when
    /// any part did. For each part's answer any.
    fn a_pass_takes_each_state_based_action_once_in_order() {
        let registry = CardRegistry::new();
        let mut state = crate::proofs::cards::new_game();

        let pass = crate::sba::one_pass(&mut state, &registry);

        // SAFETY: as in `is_creature`.
        let (called, n, answered, handed_on) = unsafe {
            (&*std::ptr::addr_of!(CALLED), *std::ptr::addr_of!(CALLED_N), &*std::ptr::addr_of!(ANSWERED), *std::ptr::addr_of!(HANDED_ON))
        };
        let last = if answered[6] { 6 } else if answered[8] { 8 } else { 10 };
        assert!(n == last + 1 && (0..n).all(|k| usize::from(called[k]) == k) && handed_on);
        let any_acted = [0, 1, 3, 4, 5, 7, 9, 10].iter().any(|&k| answered[k]);
        assert!(pass == if last < 10 {
            crate::sba::Pass::Stopped
        } else if any_acted {
            crate::sba::Pass::Acted
        } else {
            crate::sba::Pass::Settled
        });
        std::mem::forget((registry, state));
    }
}

/// The passes taken, and what each came to.
static mut PASSES: [Option<crate::sba::Pass>; 3] = [None; 3];
static mut PASSES_N: usize = 0;

/// Stands in for `one_pass`: any outcome, but settled the third time.
#[allow(dead_code)] // named only by `kani::stub`
fn a_pass(_: &mut GameState, _: &CardRegistry) -> crate::sba::Pass {
    use crate::sba::Pass;
    // SAFETY: as in `is_creature`, for PASSES and PASSES_N.
    let (passes, n) = unsafe { (&mut *std::ptr::addr_of_mut!(PASSES), &mut *std::ptr::addr_of_mut!(PASSES_N)) };
    let pass = match kani::any::<u8>() {
        _ if *n == 2 => Pass::Settled,
        0 => Pass::Settled,
        1 => Pass::Stopped,
        _ => Pass::Acted,
    };
    passes[*n] = Some(pass);
    *n += 1;
    pass
}

card_proof! {
    #[kani::stub(crate::sba::one_pass, a_pass)]
    /// State-based actions are checked until none applies (CR 704.3):
    /// pass after pass, until one comes to nothing or stops for a trigger
    /// or a player's answer; whether any action was taken is whether any
    /// pass acted or one stopped. While a choice is waiting in the middle
    /// of a resolution, no pass is taken at all (608.2: nobody would
    /// receive priority), but for the choice of what a permanent enters as
    /// a copy of, made before it is on the battlefield (614.12b). For no
    /// question waiting, an attack waiting to be declared, a "you may" in a
    /// resolution and the choice of a copy, and each pass's outcome any.
    fn state_based_actions_are_checked_until_none_applies_but_in_a_resolution() {
        use crate::sba::Pass;
        use crate::state::{AwaitingAction, ResolutionChoiceKind};
        let registry = CardRegistry::new();
        let mut state = crate::proofs::cards::new_game();
        let waiting = kani::any::<u8>() % 4;
        state.awaiting_action = match waiting {
            0 => None,
            1 => Some(AwaitingAction::DeclareAttackers),
            2 => Some(AwaitingAction::ResolutionChoice {
                player: PlayerId(0),
                source: ObjectId(1),
                choice: ResolutionChoiceKind::YesNo { description: String::new(), source_card: ObjectId(1) },
            }),
            _ => Some(AwaitingAction::ResolutionChoice {
                player: PlayerId(0),
                source: ObjectId(1),
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: String::new(),
                    options: Vec::new(),
                    optional: false,
                    effect: PendingEffect::EnterAsCopy { object: ObjectId(1) },
                },
            }),
        };

        let any = crate::sba::check_state_based_actions(&mut state, &registry);

        // SAFETY: as in `is_creature`.
        let (passes, n) = unsafe { (&*std::ptr::addr_of!(PASSES), *std::ptr::addr_of!(PASSES_N)) };
        if waiting == 2 {
            assert!(!any && n == 0);
        } else {
            assert!(n >= 1 && (0..n - 1).all(|k| passes[k] == Some(Pass::Acted)) && passes[n - 1] != Some(Pass::Acted));
            assert!(any == (n > 1 || passes[n - 1] == Some(Pass::Stopped)));
        }
        std::mem::forget((registry, state));
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, crate::proofs::cards::record_move)]
#[kani::stub(crate::state::GameState::is_legendary, is_legendary)]
#[kani::stub(crate::state::GameState::name_of, name_of)]
#[kani::stub(crate::destruction::death_event, death_event)]
/// The legend rule's answer (`engine::apply_pending_effect` of
/// `LegendRuleKeep`, CR 704.5j, 700.4): of the permanents the player
/// controls that are legendary and of the name, each but the one kept is
/// put into its owner's graveyard, in the board's order, its death
/// captured as for any permanent put there from the battlefield (and with
/// it, that a creature died this turn); the one kept, and every permanent
/// of another name or player, or not legendary, or not on the
/// battlefield, stays. For three objects, each either player's, on the
/// battlefield or in a graveyard, legendary or not and named "A" or "B",
/// the one kept any of them, the player either player and the name
/// either name.
fn the_legend_kept_stays_and_the_rest_go_to_the_graveyard() {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let (mut on, mut controller) = ([false; 3], [PlayerId(0); 3]);
    for i in 0..3 {
        on[i] = kani::any();
        controller[i] = PlayerId(u8::from(kani::any::<bool>()));
        state.create_object(blank, controller[i], if on[i] { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    }
    let kept = ObjectId(1 + u64::from(kani::any::<u8>() % 3));
    let player = PlayerId(u8::from(kani::any::<bool>()));
    let b: bool = kani::any();
    let effect = PendingEffect::LegendRuleKeep { player, legend_name: if b { "B" } else { "A" }.into() };
    let events = state.events.len();

    crate::engine::apply_pending_effect(&mut state, &Target::Object(kept), &effect, &registry);

    let mut gone = [ObjectId(0); 3];
    let mut n = 0;
    for i in 0..3 {
        let id = ObjectId(i as u64 + 1);
        if on[i] && controller[i] == player && named_b(id) == b && id != kept && is_legendary(&state, id, &registry) {
            gone[n] = id;
            n += 1;
        }
    }
    assert!(asked_each(n, |k| Act::Move(gone[k], Zone::Graveyard)));
    assert!(state.events.len() == events + n);
    for (k, &id) in gone.iter().enumerate().take(n) {
        assert!(matches!(state.events[events + k], GameEvent::Tapped { object } if object == id));
    }
    assert!(state.creature_died_this_turn == (n > 0));
    std::mem::forget((registry, state, effect));
}

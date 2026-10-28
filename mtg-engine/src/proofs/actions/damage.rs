//! Dealing damage, which a card proof's `record_damage`, `record_queue`
//! and `record_deal_queued` stand in for: `damage::deal_damage`, and
//! `damage::queue_damage` with `damage::process_pending_damage`, which a
//! spell dealing damage to several at once (CR 120.1) uses.
//!
//! Here, damage that no prevention or replacement effect applies to: which
//! ones apply (`applicable_effects`) is answered with none. Whether the
//! source has deathtouch or lifelink is any answer (`proofs::decide`), the
//! same each time it is asked; which objects have them is `has_keyword`'s
//! own proof. `damage::perform`, which deals an event once every effect on
//! it is settled, is proved on its own, and recorded in the proofs of the
//! queue around it.

use crate::cards::CardRegistry;
use crate::damage::{self, DamageEffect, DamageKind, PendingDamage};
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{blank_board, Snapshot};
use crate::proofs::decide::{decided, Question};
use crate::state::GameState;
use crate::types::{CardType, CounterType, Keyword, Zone};

/// Stands in for `damage::applicable_effects`: none applies.
#[allow(dead_code)] // named only by `kani::stub`
fn none_applies(_: &GameState, _: &PendingDamage, _: &CardRegistry) -> Vec<DamageEffect> {
    Vec::new()
}

/// Stands in for `GameState::has_keyword`: dealing damage asks only about
/// deathtouch and lifelink.
#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    match keyword {
        Keyword::Deathtouch => decided(Question::HasDeathtouch, id),
        Keyword::Lifelink => decided(Question::HasLifelink, id),
        _ => panic!("dealing damage asks only about deathtouch and lifelink"),
    }
}

/// What `perform` was asked to deal: how often, and the last event, as its
/// source, target, amount and kind.
type Performed = (usize, Option<(ObjectId, DamageTarget, u32, DamageKind)>);
static mut PERFORMED: Performed = (0, None);

/// Stands in for `damage::perform`, which deals an event's damage once
/// every effect on it is settled (`performing`, below, proves what it
/// does): recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn record_perform(_: &mut GameState, pd: PendingDamage, _: &CardRegistry) {
    // SAFETY: a proof runs on one thread, and only this module names it.
    unsafe {
        let p = &mut *std::ptr::addr_of_mut!(PERFORMED);
        *p = (p.0 + 1, Some((pd.source, pd.target, pd.amount, pd.kind)));
    }
    std::mem::forget(pd);
}

/// "N damage to any target", combat or not, that nothing prevents or
/// replaces (CR 120.3, 616.1): it is performed once, as it was queued,
/// with its source, target, amount and kind, and nothing is left waiting
/// or changed besides. A source dealing 0 queues nothing (120.8), and
/// damage to an object no longer on the battlefield is dropped unperformed
/// (120.1). Through `deal_damage`, or if `queued` through `queue_damage`
/// then `process_pending_damage`; to a creature of either player's on the
/// battlefield or not if `to` is `None`, else to that player; up to 20
/// damage. What performing it does is `performing`'s.
///
/// The queue is proved with `perform` recorded, and `perform` on its own:
/// damage of an amount the solver chose may be 0, which is never queued,
/// so the queue's length was one it left open, the loop over the queue
/// went on past its end, and each pass performed an event of every kind,
/// out of memory at six gigabytes.
fn queueing(queued: bool, to: Option<PlayerId>) {
    let (registry, mut state, blank) = blank_board();
    let source = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    let on: bool = kani::any();
    let t = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())),
        if on { Zone::Battlefield } else { Zone::Graveyard }, Some(3), Some(3));
    let target = to.map_or(DamageTarget::Object(t), DamageTarget::Player);
    let amount: u32 = kani::any();
    kani::assume(amount <= 20);
    let kind = if kani::any() { DamageKind::Combat } else { DamageKind::NonCombat };
    let before = Snapshot::of(&state);

    if queued {
        damage::queue_damage(&mut state, source, target, amount, kind);
        damage::process_pending_damage(&mut state, &registry);
    } else {
        damage::deal_damage(&mut state, source, target, amount, kind, &registry);
    }

    let reaches = match target {
        DamageTarget::Object(_) => on,
        DamageTarget::Player(_) => true,
    };
    let expected = (amount > 0 && reaches).then_some((source, target, amount, kind));
    // SAFETY: as in `record_perform`.
    let performed = unsafe { *std::ptr::addr_of!(PERFORMED) };
    assert!(performed == (usize::from(expected.is_some()), expected));
    assert!(state.pending_damage.is_empty() && Snapshot::of(&state) == before);
    kani::cover!(expected.is_some(), "performed");
    std::mem::forget((registry, state));
}

// The queue's loops run once for the one event queued, and once more to
// find the end: unwound to the start bound, symbolic execution, which
// cannot see the end of a queue whose length it leaves open (an amount of
// 0 is never queued), went on past it twice more.
// loop bound: damage::process_pending_damage 2 nothing_prevents

/// `damage::perform`: an event's damage dealt (CR 120.3). To a player, that
/// much life lost (120.3a); to a planeswalker, that many loyalty counters
/// removed, as many as it has if fewer (120.3c); to any other permanent,
/// that much damage marked on it (120.3e), and marked as deathtouch damage
/// if the source has deathtouch (702.2b); the source is then among what
/// has damaged it. The damage is announced as combat damage or not, once,
/// and if the source has lifelink its controller gains that much life
/// (702.15b). No damage is dealt at all by a source dealing 0 (120.8), or
/// to an object no longer on the battlefield. For a source `sc` controls,
/// on the battlefield or not; to a creature, or if `walker` a
/// planeswalker, of either player's, with damage or loyalty on it and
/// damaged by the source before if `damaged_before`, if `to` is `None`,
/// else to that player; either player at any life an `i16` holds, and up
/// to 20 damage. Nothing else changes.
fn performing(to: Option<PlayerId>, sc: PlayerId, walker: bool, damaged_before: bool) {
    let (registry, mut state, blank) = blank_board();
    let totals: [i16; 2] = [kani::any(), kani::any()];
    state.players[0].life = i32::from(totals[0]);
    state.players[1].life = i32::from(totals[1]);
    let source = state.create_object(blank, sc, if kani::any() { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
    let tc = PlayerId(u8::from(kani::any::<bool>()));
    let on: bool = kani::any();
    let t = state.create_object(blank, tc, if on { Zone::Battlefield } else { Zone::Graveyard }, Some(3), Some(3));
    let (marked, loyalty): (u32, u32) = (kani::any(), kani::any());
    kani::assume(marked <= 20 && loyalty <= 3);
    let o = state.get_object_mut(t).expect("just created");
    if walker {
        o.card_types = vec![CardType::Planeswalker];
        o.power = None;
        o.toughness = None;
        if loyalty > 0 {
            o.counters.insert(CounterType::Loyalty, loyalty);
        }
    } else {
        o.damage_marked = marked;
    }
    o.damaged_by = Vec::with_capacity(2);
    if damaged_before {
        o.damaged_by.push(source);
    }
    let target = to.map_or(DamageTarget::Object(t), DamageTarget::Player);
    let amount: u32 = kani::any();
    kani::assume(amount <= 20);
    let kind = if kani::any() { DamageKind::Combat } else { DamageKind::NonCombat };
    let before = Snapshot::of(&state);

    damage::perform(&mut state, PendingDamage { source, target, amount, kind, applied: Vec::new(), settled: true }, &registry);

    let dealt = amount > 0 && match target {
        DamageTarget::Object(_) => on,
        DamageTarget::Player(_) => true,
    };
    if !dealt {
        assert!(state.events.is_empty() && Snapshot::of(&state) == before);
        let o = state.get_object(t).expect("on the board");
        assert!(o.damaged_by.len() == usize::from(damaged_before) && !o.dealt_deathtouch_damage);
        std::mem::forget((registry, state));
        return;
    }
    let n = i32::try_from(amount).expect("small");
    let mut expected = before.without_events();
    let mut events = 0;
    match target {
        DamageTarget::Player(p) => {
            let old = i32::from(totals[usize::from(p.0)]);
            assert!(matches!(state.events[0], GameEvent::LifeChanged { player, old: o, new_life } if player == p && o == old && new_life == old - n));
            expected = expected.with_life(p, old - n);
            events += 1;
        }
        DamageTarget::Object(_) => {
            let o = state.get_object(t).expect("on the board");
            if walker {
                expected = expected.with_counters(t, CounterType::Loyalty, loyalty.saturating_sub(amount));
                assert!(loyalty > amount || !o.counters.contains_key(&CounterType::Loyalty));
                assert!(!o.dealt_deathtouch_damage);
            } else {
                expected = expected.with_damage(t, marked + amount);
                assert!(o.dealt_deathtouch_damage == decided(Question::HasDeathtouch, source));
            }
            assert!(o.damaged_by.len() == 1 && o.damaged_by[0] == source);
        }
    }
    assert!(match (kind, &state.events[events]) {
        (DamageKind::Combat, GameEvent::CombatDamageDealt { source: s, target: d, amount: a })
        | (DamageKind::NonCombat, GameEvent::NonCombatDamageDealt { source: s, target: d, amount: a }) =>
            *s == source && *d == target && *a == amount,
        _ => false,
    });
    events += 1;
    if decided(Question::HasLifelink, source) {
        let old = match target {
            DamageTarget::Player(p) if p == sc => i32::from(totals[usize::from(sc.0)]) - n,
            _ => i32::from(totals[usize::from(sc.0)]),
        };
        assert!(matches!(state.events[events], GameEvent::LifeChanged { player, old: o, new_life } if player == sc && o == old && new_life == old + n));
        expected = expected.with_life(sc, old + n);
        events += 1;
    }
    assert!(state.events.len() == events && Snapshot::of(&state).without_events() == expected);
    kani::cover!(walker && on && loyalty > 0 && amount >= loyalty, "a planeswalker loses its last loyalty");
    kani::cover!(matches!(target, DamageTarget::Player(p) if p == sc) && decided(Question::HasLifelink, source), "lifelink on its own controller");
    std::mem::forget((registry, state));
}

/// `performing`, a harness for each target, source's controller and
/// permanent damaged, each a constant: a player the solver chose is an
/// index into the players it leaves open, which life lost and lifelink's
/// life gained are written through, and the proof ran out of memory; a
/// permanent damaged a planeswalker or not, and damaged by the source
/// before or not, is each the length of a list (its card types, what has
/// damaged it) the solver otherwise left open, and the proof ran out of
/// memory at four gigabytes; and the cases of one harness add up.
macro_rules! each_target {
    ($($name:ident: $to:expr, $sc:literal, $walker:literal, $before:literal;)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::damage::applicable_effects, none_applies)]
        #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
        /// The proof above it, for one target and board.
        fn $name() {
            performing($to, PlayerId($sc), $walker, $before);
        }
    )*};
}

each_target! {
    damage_performed_on_a_creature_undamaged_from_a_source_of_the_first_players_is_marked_or_takes_loyalty: None, 0, false, false;
    damage_performed_on_a_creature_undamaged_from_a_source_of_the_second_players_is_marked_or_takes_loyalty: None, 1, false, false;
    damage_performed_on_a_creature_damaged_by_it_before_from_a_source_of_the_first_players_is_marked_or_takes_loyalty: None, 0, false, true;
    damage_performed_on_a_creature_damaged_by_it_before_from_a_source_of_the_second_players_is_marked_or_takes_loyalty: None, 1, false, true;
    damage_performed_on_a_planeswalker_undamaged_from_a_source_of_the_first_players_is_marked_or_takes_loyalty: None, 0, true, false;
    damage_performed_on_a_planeswalker_undamaged_from_a_source_of_the_second_players_is_marked_or_takes_loyalty: None, 1, true, false;
    damage_performed_on_a_planeswalker_damaged_by_it_before_from_a_source_of_the_first_players_is_marked_or_takes_loyalty: None, 0, true, true;
    damage_performed_on_a_planeswalker_damaged_by_it_before_from_a_source_of_the_second_players_is_marked_or_takes_loyalty: None, 1, true, true;
    damage_performed_on_the_first_player_from_a_source_of_the_first_players_is_life_lost: Some(PlayerId(0)), 0, false, false;
    damage_performed_on_the_first_player_from_a_source_of_the_second_players_is_life_lost: Some(PlayerId(0)), 1, false, false;
    damage_performed_on_the_second_player_from_a_source_of_the_first_players_is_life_lost: Some(PlayerId(1)), 0, false, false;
    damage_performed_on_the_second_player_from_a_source_of_the_second_players_is_life_lost: Some(PlayerId(1)), 1, false, false;
}

/// The proof of the queue above, a harness for each path and each target,
/// with `perform` recorded: a player the solver chose is an index into the
/// players it leaves open, and the cases of one harness add up.
macro_rules! each_path {
    ($($name:ident: $queued:expr, $to:expr;)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::damage::applicable_effects, none_applies)]
        #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
        #[kani::stub(crate::damage::perform, record_perform)]
        /// The proof of the queue above, for one path and one target.
        fn $name() {
            queueing($queued, $to);
        }
    )*};
}

each_path! {
    dealt_damage_to_a_permanent_nothing_prevents_is_performed_once_as_queued: false, None;
    dealt_damage_to_the_first_player_nothing_prevents_is_performed_once_as_queued: false, Some(PlayerId(0));
    dealt_damage_to_the_second_player_nothing_prevents_is_performed_once_as_queued: false, Some(PlayerId(1));
    queued_damage_to_a_permanent_nothing_prevents_is_performed_once_when_processed: true, None;
    queued_damage_to_the_first_player_nothing_prevents_is_performed_once_when_processed: true, Some(PlayerId(0));
    queued_damage_to_the_second_player_nothing_prevents_is_performed_once_when_processed: true, Some(PlayerId(1));
}

/// Stands in for `GameState::has_protection_from`: any answer, the same
/// each time for the same object (`proofs::decide`); which objects have
/// protection from what is its own proof's.
#[allow(dead_code)] // named only by `kani::stub`
fn has_protection_from(_: &GameState, target: ObjectId, _: ObjectId, _: &CardRegistry) -> bool {
    decided(Question::HasProtection, target)
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::has_protection_from, has_protection_from)]
/// Which prevention and replacement effects apply to damage (CR 615,
/// 616.1), on a board whose cards print none and with none set up this
/// turn: only protection's, which prevents all damage from a source the
/// target has protection from (702.16e); a player here has protection
/// from nothing. For damage combat or not, from either of two permanents
/// to the other or to either player, with protection or without.
fn on_a_board_printing_none_only_protection_applies() {
    let (registry, mut state, blank) = blank_board();
    let a = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    let b = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    let (source, other) = if kani::any() { (a, b) } else { (b, a) };
    let target = if kani::any() { DamageTarget::Object(other) } else { DamageTarget::Player(PlayerId(u8::from(kani::any::<bool>()))) };
    let pd = PendingDamage {
        source, target, amount: 3, kind: if kani::any() { DamageKind::Combat } else { DamageKind::NonCombat },
        applied: Vec::new(), settled: false,
    };

    let effects = damage::applicable_effects(&state, &pd, &registry);

    let protected = matches!(target, DamageTarget::Object(t) if decided(Question::HasProtection, t));
    if protected {
        assert!(effects.len() == 1 && matches!(effects[0], DamageEffect::PreventAll(damage::Prevention::Protection)));
    } else {
        assert!(effects.is_empty());
    }
    std::mem::forget((registry, state, pd, effects));
}

/// Which of protection, a shield that removes a counter and a doubler
/// apply to the damage in this proof: set once, before it is dealt.
static mut WHICH: [bool; 3] = [false; 3];

/// Stands in for `damage::applicable_effects`: the same choice among
/// protection's prevention, `shield`'s, and `flail`'s doubling, each time
/// it is asked, less those already applied to the damage, in the order
/// `applicable_effects` lists them (preventions, then doubling).
#[allow(dead_code)] // named only by `kani::stub`
fn some_effects(_: &GameState, pd: &PendingDamage, _: &CardRegistry) -> Vec<DamageEffect> {
    // SAFETY: a proof runs on one thread, and only this module names it.
    let which = unsafe { *std::ptr::addr_of!(WHICH) };
    let all = [
        DamageEffect::PreventAll(damage::Prevention::Protection),
        DamageEffect::PreventAndRemoveCounter { by: SHIELD },
        DamageEffect::Double { by: FLAIL },
    ];
    let mut out = Vec::with_capacity(3);
    for (e, on) in all.into_iter().zip(which) {
        if on && !pd.applied.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// The permanents whose effects `some_effects` names.
const SHIELD: ObjectId = ObjectId(1);
const FLAIL: ObjectId = ObjectId(2);

/// Prevention and replacement (CR 615, 616.1): damage that protection or
/// a shield prevents is not dealt, the shield taking a +1/+1 counter off
/// its permanent if it has one; damage a doubler doubles is performed
/// twice over, once (a doubled event is not doubled again), and nothing
/// else changes before it is (what performing it does is `performing`'s);
/// and where the
/// effects that apply might come out differently in different orders (a
/// shield's counter is removed or not as protection goes first or not),
/// the player the damage would be dealt to is asked, once, which applies
/// first, and nothing is dealt until they answer. Where every order comes
/// out the same (protection and a doubler, a shield and a doubler), nobody
/// is asked. For `which` of the three apply, combat damage of 1 to 3 to
/// `p`, and a shield with a counter or not; `perform` recorded, as
/// `queueing` says.
fn preventing_and_doubling(which: [bool; 3], p: PlayerId) {
    // SAFETY: a proof runs on one thread, and only this module names it.
    unsafe { *std::ptr::addr_of_mut!(WHICH) = which };
    let (registry, mut state, blank) = blank_board();
    let shield = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
    let flail = state.create_object(blank, PlayerId(0), Zone::Battlefield, None, None);
    assert!(shield == SHIELD && flail == FLAIL);
    let countered: bool = kani::any();
    if countered {
        state.get_object_mut(shield).expect("just created").counters.insert(crate::types::CounterType::PlusOnePlusOne, 1);
    }
    let source = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    let amount: u32 = kani::any();
    kani::assume((1..=3).contains(&amount));
    let before = Snapshot::of(&state);

    damage::deal_damage(&mut state, source, DamageTarget::Player(p), amount, DamageKind::Combat, &registry);

    let [protection, shielded, doubled] = which;
    // SAFETY: as in `record_perform`.
    let performed = unsafe { *std::ptr::addr_of!(PERFORMED) };
    if protection && shielded {
        // Protection first leaves the shield's counter; the shield first
        // takes it, if there is one to take (and with none, the player is
        // asked all the same, as they may be: 616.1).
        assert!(state.events.is_empty() && Snapshot::of(&state) == before.with_asking(true).with_pending_damage(1));
        assert!(matches!(&state.awaiting_action, Some(crate::state::AwaitingAction::ResolutionChoice {
            player, choice: crate::state::ResolutionChoiceKind::ChooseDamageEffect { effects, .. }, ..
        }) if *player == p && effects.len() == 2 + usize::from(doubled)));
        assert!(state.pending_damage.len() == 1 && performed == (0, None));
    } else if protection || shielded {
        // Prevented, by whichever is listed first; the counter, if the
        // shield prevented it.
        let took = !protection && countered;
        let expected = if took { before.with_counters(shield, crate::types::CounterType::PlusOnePlusOne, 0) } else { before };
        assert!(state.events.is_empty() && Snapshot::of(&state) == expected && state.pending_damage.is_empty());
        assert!(performed == (0, None));
    } else {
        let dealt = if doubled { 2 * amount } else { amount };
        assert!(state.pending_damage.is_empty() && state.events.is_empty() && Snapshot::of(&state) == before);
        assert!(performed == (1, Some((source, DamageTarget::Player(p), dealt, DamageKind::Combat))));
    }
    kani::cover!(doubled && !protection && !shielded, "doubled and dealt");
    kani::cover!(protection && shielded && countered, "the player is asked, with a counter at stake");
    std::mem::forget((registry, state));
}

/// The proof of prevention and doubling above, for each set of the three
/// that applies and each player damaged, each a constant: a set the solver
/// left open was a list of effects of every length, each order of it
/// compared, and the proof ran out of memory at four gigabytes even with
/// the player a constant; and the cases of one harness add up.
macro_rules! each_set {
    ($($name:ident: $which:expr, $p:literal;)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::damage::applicable_effects, some_effects)]
        #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
        #[kani::stub(crate::damage::perform, record_perform)]
        /// The proof of prevention and doubling, for one set of effects and
        /// one player damaged.
        fn $name() {
            preventing_and_doubling($which, PlayerId($p));
        }
    )*};
}

each_set! {
    damage_nothing_applies_to_is_performed_as_it_was_to_the_first_player: [false, false, false], 0;
    damage_nothing_applies_to_is_performed_as_it_was_to_the_second_player: [false, false, false], 1;
    damage_a_doubler_doubles_is_performed_doubled_once_to_the_first_player: [false, false, true], 0;
    damage_a_doubler_doubles_is_performed_doubled_once_to_the_second_player: [false, false, true], 1;
    damage_a_shield_prevents_is_not_performed_to_the_first_player: [false, true, false], 0;
    damage_a_shield_prevents_is_not_performed_to_the_second_player: [false, true, false], 1;
    damage_a_shield_and_a_doubler_apply_to_is_prevented_unasked_to_the_first_player: [false, true, true], 0;
    damage_a_shield_and_a_doubler_apply_to_is_prevented_unasked_to_the_second_player: [false, true, true], 1;
    damage_protection_prevents_is_not_performed_to_the_first_player: [true, false, false], 0;
    damage_protection_prevents_is_not_performed_to_the_second_player: [true, false, false], 1;
    damage_protection_and_a_doubler_apply_to_is_prevented_unasked_to_the_first_player: [true, false, true], 0;
    damage_protection_and_a_doubler_apply_to_is_prevented_unasked_to_the_second_player: [true, false, true], 1;
    protection_and_a_shield_ask_the_damaged_player_which_first_to_the_first_player: [true, true, false], 0;
    protection_and_a_shield_ask_the_damaged_player_which_first_to_the_second_player: [true, true, false], 1;
    protection_a_shield_and_a_doubler_ask_the_damaged_player_which_first_to_the_first_player: [true, true, true], 0;
    protection_a_shield_and_a_doubler_ask_the_damaged_player_which_first_to_the_second_player: [true, true, true], 1;
}

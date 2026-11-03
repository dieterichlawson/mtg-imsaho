//! Declaring attackers (`engine::actions::combat::declare_attackers` and
//! `combat::declare_attackers`, CR 508.1): what makes a creature attacking,
//! and the event the attack triggers are raised from (`triggering`).
//!
//! What is asked of each creature is any answer here, the same each time
//! it is asked: whether it is a creature, has summoning sickness, has
//! defender or vigilance, may attack (an Aura's "can't attack") and must
//! (an "attacks each combat"), and whether it is tapped. Each question is
//! its own proof (`predicates`, `effects`); these are of how the
//! declaration puts the answers together.

// The lists a declaration asks `contains` of (the creatures that may
// attack, those named, those already attacking) are of a length the
// answers decide, at most the board's two: each walk of one bounded to
// three passes, the unwinding assertion proving no fourth is taken.
// loop bound: <std::slice::Iter<'_, ids::ObjectId> as std::iter::Iterator>::any::<{closure@<ids::ObjectId as core::slice::cmp::SliceContains>::slice_contains::{closure#0}}> 3
// loop bound: <std::iter::Copied<std::slice::Iter<'_, ids::ObjectId>> as std::iter::Iterator>::try_fold::* 3
// loop bound: <std::slice::Iter<'_, ids::ObjectId> as std::iter::Iterator>::try_fold::* 3

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{asked_each, card_proof, Act, Blank};
use crate::state::GameState;
use crate::types::{Keyword, Zone};

/// What is asked of a creature.
#[derive(Clone, Copy)]
enum Asked {
    Creature,
    Sick,
    Defender,
    Vigilance,
    MayAttack,
    MustAttack,
}

/// The answers, by creature (the board's objects 1 and 2) and question.
static mut ANSWERS: [[Option<bool>; 6]; 2] = [[None; 6]; 2];

fn answer(id: ObjectId, asked: Asked) -> bool {
    let i = match id.0 {
        1 => 0,
        2 => 1,
        _ => panic!("a creature this proof does not make"),
    };
    // SAFETY: a proof runs on one thread, and only this function names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS) };
    *answers[i][asked as usize].get_or_insert_with(kani::any::<bool>)
}

#[allow(dead_code)] // named only by `kani::stub`
fn is_creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::Creature)
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_summoning_sickness(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::Sick)
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    match keyword {
        Keyword::Defender => answer(id, Asked::Defender),
        Keyword::Vigilance => answer(id, Asked::Vigilance),
        _ => panic!("a keyword the declaration does not ask"),
    }
}

#[allow(dead_code)] // named only by `kani::stub`
fn can_attack(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::MayAttack)
}

#[allow(dead_code)] // named only by `kani::stub`
fn must_attack(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    answer(id, Asked::MustAttack)
}

/// The active player's two permanents (1 and 2), each tapped or not, on a
/// board whose active player is either player.
fn board() -> (CardRegistry, GameState, PlayerId, [bool; 2]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let active = PlayerId(u8::from(kani::any::<bool>()));
    state.active_player = active;
    let mut tapped = [false; 2];
    for t in &mut tapped {
        let id = state.create_object(blank, active, Zone::Battlefield, Some(1), Some(1));
        *t = kani::any();
        state.get_object_mut(id).expect("just created").tapped = *t;
    }
    (registry, state, active, tapped)
}

/// Who attacks, of a declaration naming `named`, each entry a creature
/// named attacking a player (`who_attacks`, CR 508.1a, 508.1d): the
/// creatures named at the defending player that may attack (a creature on
/// the battlefield, untapped, without summoning sickness or defender, that
/// nothing stops attacking: 508.1a, 302.6, 702.3b), each once however many
/// times it is named, in the order named; and each other one that may
/// attack and must, in the board's order. For each permanent tapped or
/// not, and `named` a constant of each harness, by creature and whether
/// at the defending player.
fn deciding<const N: usize>(named: [(u64, bool); N]) {
    let (registry, state, active, tapped) = board();
    let defending = state.opponent(active);
    let submitted: [(ObjectId, PlayerId); N] = named.map(|(id, at_them)| (ObjectId(id), if at_them { defending } else { active }));

    let attack = crate::engine::actions::combat::who_attacks(&state, &submitted, &[], &registry);

    let may = |id: ObjectId| answer(id, Asked::Creature) && !tapped[(id.0 - 1) as usize] && !answer(id, Asked::Sick)
        && !answer(id, Asked::Defender) && answer(id, Asked::MayAttack);
    let mut chosen = [ObjectId(0); 2];
    let mut n = 0;
    for &(id, at_them) in &named {
        let id = ObjectId(id);
        if at_them && may(id) && !chosen[..n].contains(&id) {
            chosen[n] = id;
            n += 1;
        }
    }
    assert!(attack.named.len() == n && attack.walkers.is_empty());
    for (k, &id) in chosen.iter().enumerate().take(n) {
        assert!(attack.named[k] == (id, defending));
    }
    let mut forced = [ObjectId(0); 2];
    let mut m = 0;
    for i in 1..=2 {
        let id = ObjectId(i);
        if may(id) && !chosen[..n].contains(&id) && answer(id, Asked::MustAttack) {
            forced[m] = id;
            m += 1;
        }
    }
    assert!(attack.forced.len() == m);
    for (k, &id) in forced.iter().enumerate().take(m) {
        assert!(attack.forced[k] == id);
    }
    std::mem::forget((registry, state, attack));
}

/// The proofs of who attacks, each question any answer as above.
macro_rules! each_declaration {
    ($($name:ident: $named:expr,)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::is_creature, is_creature)]
            #[kani::stub(crate::state::GameState::has_summoning_sickness, has_summoning_sickness)]
            #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
            #[kani::stub(crate::state::GameState::can_attack, can_attack)]
            #[kani::stub(crate::state::GameState::must_attack, must_attack)]
            /// [`deciding`] for the declaration of its name.
            fn $name() {
                deciding($named);
            }
        }
    )*};
}

each_declaration! {
    declaring_no_attacker_attacks_with_those_that_must: [],
    declaring_one_attacker_at_the_defending_player: [(1, true)],
    declaring_one_attacker_at_its_own_controller_attacks_with_none_but_those_that_must: [(1, false)],
    declaring_one_attacker_twice_attacks_with_it_once: [(1, true), (1, true)],
    declaring_both_attackers_attacks_with_them_in_the_order_named: [(2, true), (1, true)],
}

/// The attack recorded (`combat::declare_attackers`, CR 508.1f, 508.1m):
/// the creatures of `order` (of the board's two) attacking the defending
/// player are exactly the combat's attackers, each with no blockers yet;
/// each is tapped unless it has vigilance (702.20b), in that order, and
/// stamped as having attacked this turn; and the attack's one event lists
/// exactly those, in that order. For each permanent with vigilance or not.
fn recording(order: &[u64]) {
    let (registry, mut state, active, _) = board();
    let defending = state.opponent(active);
    let turn = state.turn_number;
    let declared: Vec<(ObjectId, PlayerId)> = order.iter().map(|&i| (ObjectId(i), defending)).collect();
    let events = state.events.len();

    crate::combat::declare_attackers(&mut state, &declared, &[], &registry);

    let combat = state.combat.as_ref().expect("a combat begun");
    assert!(combat.attackers.len() == order.len() && combat.planeswalker_defenders.is_empty());
    assert!(matches!(&state.events[events..], [GameEvent::AttackersDeclared { attackers }] if attackers.len() == order.len()
        && attackers.iter().zip(order).all(|(&(a, d), &i)| a == ObjectId(i) && d == defending)));
    for &i in order {
        let id = ObjectId(i);
        assert!(combat.attackers.get(&id) == Some(&defending));
        assert!(combat.blocker_assignments.get(&id).is_some_and(Vec::is_empty));
        assert!(state.get_object(id).expect("on the board").attacked_on_turn == Some(turn));
    }
    let mut tapping = [ObjectId(0); 2];
    let mut t = 0;
    for &i in order {
        let id = ObjectId(i);
        if !answer(id, Asked::Vigilance) {
            tapping[t] = id;
            t += 1;
        }
    }
    assert!(asked_each(t, |k| Act::Tap(tapping[k])));
    std::mem::forget((registry, state, declared));
}

/// The proofs of recording an attack, vigilance any answer.
macro_rules! each_recording {
    ($($name:ident: $order:expr,)*) => {$(
        card_proof! {
            #[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
            /// [`recording`] for the attackers of its name.
            fn $name() {
                recording(&$order);
            }
        }
    )*};
}

each_recording! {
    an_attack_of_none_is_recorded_as_none: [],
    an_attack_of_one_is_recorded_tapping_it_unless_it_has_vigilance: [2],
    an_attack_of_two_is_recorded_in_the_order_declared: [2, 1],
}

/// What [`composing`] finds recorded: the attackers and the planeswalker
/// attacks handed to `combat::declare_attackers`.
static mut RECORDED: Option<(Vec<(ObjectId, PlayerId)>, usize)> = None;

/// Stands in for `who_attacks`: one creature named, none at a
/// planeswalker, and the other forced.
#[allow(dead_code)] // named only by `kani::stub`
fn decided(state: &GameState, _: &[(ObjectId, PlayerId)], _: &[(ObjectId, ObjectId)], _: &CardRegistry) -> crate::engine::actions::combat::Attack {
    let defending = state.opponent(state.active_player);
    crate::engine::actions::combat::Attack { named: vec![(ObjectId(2), defending)], walkers: Vec::new(), forced: vec![ObjectId(1)] }
}

/// Stands in for `combat::declare_attackers`: records what it is handed.
#[allow(dead_code)] // named only by `kani::stub`
fn record_declared(_: &mut GameState, attackers: &[(ObjectId, PlayerId)], walkers: &[(ObjectId, ObjectId)], _: &CardRegistry) {
    // SAFETY: as in `answer`.
    unsafe { *std::ptr::addr_of_mut!(RECORDED) = Some((attackers.to_vec(), walkers.len())) };
}

card_proof! {
    #[kani::stub(crate::engine::actions::combat::who_attacks, decided)]
    #[kani::stub(crate::combat::declare_attackers, record_declared)]
    /// The declaration as a whole: what is recorded attacking is who
    /// attacks, the creatures named first, in their order, then those
    /// forced, all at the defending player, with the planeswalker attacks
    /// beside them; and no prompt is left waiting.
    fn the_attack_recorded_is_the_one_decided_named_first_then_forced() {
        let (registry, mut state, active, _) = board();
        let defending = state.opponent(active);
        crate::engine::actions::combat::declare_attackers(&mut state, &[], &[], &registry);
        // SAFETY: as in `answer`.
        let recorded = unsafe { &*std::ptr::addr_of!(RECORDED) };
        assert!(matches!(recorded, Some((a, 0)) if a.as_slice() == [(ObjectId(2), defending), (ObjectId(1), defending)]));
        assert!(state.awaiting_action.is_none());
        std::mem::forget((registry, state));
    }
}

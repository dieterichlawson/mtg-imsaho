//! A permanent that may enter as a copy of a creature (Evil Twin, CR
//! 614.12b, 707.9): its entry waits on the choice, which is asked before
//! anything else looks at the board (`replacement::
//! process_pending_entry_choices`), pruned of creatures gone since it was
//! asked (`refresh_pending_entry_choice`), and, answered, finishes the
//! entry (`record_entry_choice`). What the permanent is once it has
//! entered as a copy is Evil Twin's own proof; the answer itself is
//! `answering`'s.

use crate::proofs::held::Held;
use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{asked_for, card_proof, Act, Blank};
use crate::state::{AwaitingAction, EnterAsCopyChoice, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

/// The answer recorded and the entry finished (`record_entry_choice`, CR
/// 614.12b, 110.2): the permanent holds the answer (a copy of the creature
/// chosen, or declined), is no longer waiting on a choice (every other
/// permanent waiting still is) nor holding what it might copy or be given,
/// and is put onto the battlefield, under the
/// player its entry was to give it to if `held` names one, and otherwise
/// as it enters by default (its owner's control). For either answer,
/// another permanent waiting or none, and `held` a constant of each
/// harness: left to the solver, the map it is held in was taken apart
/// every way a map of it might be.
fn recording(held: Option<PlayerId>) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let entering = state.create_object(blank, PlayerId(0), Zone::Stack, Some(0), Some(0));
    let copied = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    let waiting = state.create_object(blank, PlayerId(1), Zone::Stack, Some(0), Some(0));
    let other_waits: bool = kani::any();
    state.pending_entry_choices.reserve(2);
    state.pending_entry_choices.push(entering);
    if other_waits {
        state.pending_entry_choices.push(waiting);
    }
    if let Some(p) = held {
        state.pending_entry_controllers.insert(entering, p);
    }
    // What it may copy and what it is given, kept with the entry.
    state.pending_entry_options.insert(entering, vec![copied]);
    state.pending_entry_grants.insert(entering, (Vec::new(), Vec::new()));
    let choice = if kani::any() { EnterAsCopyChoice::Copy(copied) } else { EnterAsCopyChoice::Declined };

    crate::replacement::record_entry_choice(&mut state, entering, choice, &registry);

    assert!(state.get_object(entering).expect("on the board").entering_copy_choice == choice);
    let one = [waiting];
    let still: &[ObjectId] = if other_waits { &one } else { &[] };
    assert!(state.pending_entry_choices.as_slice() == still);
    assert!(state.pending_entry_controllers.is_empty() && state.pending_entry_options.is_empty() && state.pending_entry_grants.is_empty());
    assert!(match held {
        Some(p) => asked_for(&[Act::MoveUnder(entering, Zone::Battlefield, p)]),
        None => asked_for(&[Act::Move(entering, Zone::Battlefield)]),
    });
    std::mem::forget((registry, state, choice));
}

card_proof! {
    /// [`recording`] of an entry with no controller held for it.
    fn an_entry_choice_recorded_finishes_the_entry_under_its_owner() {
        recording(None);
    }
}

card_proof! {
    /// [`recording`] of an entry held for the other player's control.
    fn an_entry_choice_recorded_finishes_the_entry_under_the_controller_held() {
        recording(Some(PlayerId(1)));
    }
}

/// Whether each of the board's objects 2 and 3 is a creature: any answer.
static mut CREATURE: Held<[Option<bool>; 2]> = Held::new([None; 2], (&raw const CREATURE).cast());

/// Stands in for `GameState::is_creature` (`predicates`): any answer for
/// objects 2 and 3, the same each time.
#[allow(dead_code)] // each named only by `kani::stub`
fn creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    let i = match id.0 {
        2 => 0,
        3 => 1,
        _ => panic!("an object this proof does not ask about"),
    };
    // SAFETY: a proof runs on one thread, and only this function names CREATURE.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(CREATURE.value) };
    *answers[i].get_or_insert_with(kani::any::<bool>)
}

/// What the entry was told, once: declined (none to copy), or asked of
/// whom, among which (up to two), and about which permanent.
static mut TOLD: Held<Option<(Option<EnterAsCopyChoice>, Option<(ObjectId, PlayerId, [Option<ObjectId>; 2], usize, bool)>)>> = Held::new(None, (&raw const TOLD).cast());

#[allow(dead_code)]
fn entry_recorded(state: &mut GameState, object: ObjectId, choice: EnterAsCopyChoice, _: &CardRegistry) {
    // SAFETY: as in `creature`, for TOLD.
    let t = unsafe { &mut *std::ptr::addr_of_mut!(TOLD.value) };
    assert!(t.is_none(), "told once");
    *t = Some((Some(choice), None));
    state.pending_entry_choices.retain(|&id| id != object);
}

#[allow(dead_code)]
fn asked(
    state: &mut GameState, source: ObjectId, player: PlayerId, options: Vec<Target>, effect: PendingEffect, _: &str, _: &CardRegistry,
) {
    let id = |i: usize| match options.get(i) {
        Some(Target::Object(o)) => Some(*o),
        _ => None,
    };
    let about = matches!(effect, PendingEffect::EnterAsCopy { object } if object == source);
    // SAFETY: as in `entry_recorded`.
    let t = unsafe { &mut *std::ptr::addr_of_mut!(TOLD.value) };
    assert!(t.is_none(), "told once");
    *t = Some((None, Some((source, player, [id(0), id(1)], options.len(), about))));
    // What asking leaves: a question waiting.
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    std::mem::forget((options, effect));
}

// Not `card_proof!`, which stands in for asking with a recorder that
// leaves no question waiting: the loop asking it would ask again.
#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::is_creature, creature)]
#[kani::stub(crate::replacement::record_entry_choice, entry_recorded)]
#[kani::stub(crate::cards::helpers::present_optional_target_choice, asked)]
/// A permanent waiting on what it enters as, asked (`process_pending_
/// entry_choices`, CR 614.12b, 115.1): its controller is asked whether
/// it enters as a copy of a creature, and of which, among exactly the
/// creatures on the battlefield, in the board's order (a choice and not
/// a target, so none is left out for hexproof or protection); with no
/// creature there to copy, it enters as its printed self, unasked; and
/// one waiting that is no longer anywhere is passed over. For a
/// permanent either player's waiting behind one gone, and two other
/// objects, each on the battlefield or not and a creature or not.
fn a_permanent_that_may_enter_as_a_copy_is_asked_among_the_creatures_there() {
    asking(false);
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::is_creature, creature)]
#[kani::stub(crate::replacement::record_entry_choice, entry_recorded)]
#[kani::stub(crate::cards::helpers::present_optional_target_choice, asked)]
/// [`asking`], for a permanent put onto the battlefield with others, which
/// kept what was there before them: object 3, object 2 having entered with
/// it (Evil Twin's ruling), so that only object 3 is offered, if it is
/// still a creature on the battlefield.
fn a_permanent_entering_with_others_is_asked_among_the_creatures_there_before_them() {
    asking(true);
}

/// [`a_permanent_that_may_enter_as_a_copy_is_asked_among_the_creatures_there`],
/// with the creatures it may copy kept with its entry when `kept`.
fn asking(kept: bool) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let entering = state.create_object(blank, controller, Zone::Stack, Some(0), Some(0));
    let mut on = [false; 2];
    for o in &mut on {
        *o = kani::any();
        state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), if *o { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
    }
    state.pending_entry_choices.reserve(2);
    state.pending_entry_choices.push(ObjectId(9));
    state.pending_entry_choices.push(entering);
    if kept {
        state.pending_entry_options.insert(entering, vec![ObjectId(3)]);
    }

    crate::replacement::process_pending_entry_choices(&mut state, &registry);

    let mut expected = [None; 2];
    let mut n = 0;
    for (i, &o) in on.iter().enumerate() {
        let id = ObjectId(i as u64 + 2);
        if o && !(kept && id == ObjectId(2)) && creature(&state, id, &registry) {
            expected[n] = Some(id);
            n += 1;
        }
    }
    // SAFETY: as in `entry_recorded`.
    let t = unsafe { *std::ptr::addr_of!(TOLD.value) };
    if n == 0 {
        assert!(t == Some((Some(EnterAsCopyChoice::Declined), None)) && state.pending_entry_choices.is_empty());
    } else {
        assert!(t == Some((None, Some((entering, controller, expected, n, true)))));
        assert!(state.pending_entry_choices.as_slice() == [entering]);
    }
    std::mem::forget((registry, state));
}

card_proof! {
    #[kani::stub(crate::state::GameState::is_creature, creature)]
    #[kani::stub(crate::replacement::record_entry_choice, entry_recorded)]
    /// The choice of what a permanent enters as, read again before it is
    /// answered (`refresh_pending_entry_choice`, CR 608.2d): the creatures
    /// offered that are still creatures on the battlefield stay offered, in
    /// order, and the rest are no longer; with none left, there is nothing
    /// to copy, and the permanent enters as its printed self, the question
    /// gone. For two creatures offered, each still there or gone, and each
    /// still a creature or not.
    fn the_creatures_a_permanent_may_enter_as_a_copy_of_are_those_still_there() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let entering = state.create_object(blank, PlayerId(0), Zone::Stack, Some(0), Some(0));
        let mut on = [false; 2];
        for o in &mut on {
            *o = kani::any();
            state.create_object(blank, PlayerId(1), if *o { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
        }
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: PlayerId(0),
            source: entering,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: String::new(),
                options: vec![Target::Object(ObjectId(2)), Target::Object(ObjectId(3))],
                optional: true,
                effect: PendingEffect::EnterAsCopy { object: entering },
            },
        });

        crate::replacement::refresh_pending_entry_choice(&mut state, &registry);

        let live = [on[0] && creature(&state, ObjectId(2), &registry), on[1] && creature(&state, ObjectId(3), &registry)];
        // SAFETY: as in `entry_recorded`.
        let t = unsafe { *std::ptr::addr_of!(TOLD.value) };
        if live == [false, false] {
            assert!(state.awaiting_action.is_none() && t == Some((Some(EnterAsCopyChoice::Declined), None)));
        } else {
            assert!(t.is_none());
            let Some(AwaitingAction::ResolutionChoice { player: PlayerId(0), choice: ResolutionChoiceKind::ChooseTarget {
                options, optional: true, effect: PendingEffect::EnterAsCopy { object }, .. }, .. }) = &state.awaiting_action
            else {
                panic!("the question still waiting");
            };
            assert!(*object == entering && options.len() == usize::from(live[0]) + usize::from(live[1]));
            for (k, t) in options.iter().enumerate() {
                let expected = if live[0] && k == 0 { ObjectId(2) } else { ObjectId(3) };
                assert!(matches!(t, Target::Object(id) if *id == expected));
            }
        }
        std::mem::forget((registry, state));
    }
}

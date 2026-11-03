//! A triggered ability going on the stack with its target chosen
//! (`triggers::push_one_pending_trigger`, CR 603.3c-d), and the answer to
//! the prompt that chooses it (`AttachTargetToPendingTrigger`): what puts
//! the target a trigger resolves with (`trigger_resolving`) on it.
//!
//! The requirement the card prints (`triggers::target_requirement`, proved
//! in `trigger_resolving`) and the targets it allows
//! (`targeting::valid_targets_for_req`, the targeting proofs' question)
//! stand in here: none, or "a creature" with exactly the board's first `n`
//! objects allowed.

use crate::actions::Target;
use crate::cards::{CardRegistry, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::Blank;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind, StackEntry};
use crate::triggers::{PendingTrigger, TriggerEvent};
use crate::types::Zone;

/// Whether the trigger targets, and how many targets it is allowed.
static mut ALLOWED: Option<usize> = None;

/// Stands in for `triggers::target_requirement`: a creature, if the
/// trigger targets.
#[allow(dead_code)] // named only by `kani::stub`
fn requirement(_: &CardRegistry, _: crate::ids::CardId, _: Option<crate::cards::TriggerKind>) -> Option<TargetRequirement> {
    // SAFETY: a proof runs on one thread; the harness writes ALLOWED first.
    unsafe { *std::ptr::addr_of!(ALLOWED) }.map(|_| TargetRequirement::Creature)
}

/// Stands in for `targeting::valid_targets_for_req`: the board's first
/// `n` objects.
#[allow(dead_code)] // named only by `kani::stub`
fn allowed(_: &GameState, _: PlayerId, _: ObjectId, _: &TargetRequirement, _: &dyn crate::cards::CardBehavior, _: &CardRegistry) -> Vec<Target> {
    // SAFETY: as in `requirement`.
    let n = unsafe { *std::ptr::addr_of!(ALLOWED) }.expect("asked only of a trigger that targets");
    let mut targets = Vec::with_capacity(2);
    for i in 1..=n {
        targets.push(Target::Object(ObjectId(i as u64)));
    }
    targets
}

/// Two creatures (the board's objects 1 and 2, either player's) and the
/// trigger's source (3), of `controller`'s, on a board whose active player
/// is `active`; the trigger, built as the collector builds it, an upkeep
/// trigger of the source's.
fn board(controller: PlayerId, active: PlayerId) -> (CardRegistry, GameState, PendingTrigger, ObjectId) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    state.active_player = active;
    for _ in 0..2 {
        state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    }
    let source = state.create_object(blank, controller, Zone::Battlefield, Some(1), Some(1));
    assert!(source == ObjectId(3));
    state.pending_trigger_pushes_ap.reserve(1);
    state.pending_trigger_pushes_nap.reserve(1);
    let mut c = crate::triggers::collect::Collector::new(controller);
    c.emit(source, blank, controller, String::new(), TriggerEvent::Upkeep);
    let trigger = c.ap.pop().expect("just emitted");
    std::mem::forget(c);
    (registry, state, trigger, source)
}

/// A trigger going on the stack (CR 603.3c-d): one that does not target,
/// as it is; one that does, with no target allowed, not at all (603.3c);
/// with one, holding it; with two, not yet: it waits at the front of its
/// controller's queue (the active player's or the other's, 603.3b) while
/// its controller is asked to choose between exactly those, the answer
/// attaching it (`attach`). For a trigger either player's on either
/// player's turn, `targets` of `None` for one that does not target.
fn pushing((targets, controller, active): (Option<usize>, PlayerId, PlayerId)) {
    // SAFETY: as in `requirement`.
    unsafe { *std::ptr::addr_of_mut!(ALLOWED) = targets };
    let (registry, mut state, trigger, source) = board(controller, active);

    crate::triggers::push_one_pending_trigger(&mut state, trigger, &registry);

    let on_stack = |n: usize| matches!(state.stack.as_slice(), [StackEntry::Trigger(t)]
        if t.source.id == source && t.source.controller == controller
            && (0..n).all(|i| t.source.chosen_targets.get(i) == Some(&Target::Object(ObjectId(i as u64 + 1))))
            && t.source.chosen_targets.len() == n);
    match targets {
        None => { assert!(on_stack(0) && state.awaiting_action.is_none()); }
        Some(0) => { assert!(state.stack.is_empty() && state.awaiting_action.is_none()); }
        Some(1) => { assert!(on_stack(1) && state.awaiting_action.is_none()); }
        Some(_) => {
            assert!(state.stack.is_empty());
            let (waiting, other) = if controller == active {
                (&state.pending_trigger_pushes_ap, &state.pending_trigger_pushes_nap)
            } else {
                (&state.pending_trigger_pushes_nap, &state.pending_trigger_pushes_ap)
            };
            assert!(other.is_empty() && matches!(waiting.as_slice(), [t] if t.source.id == source && t.source.chosen_targets.is_empty()));
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source: s,
                choice: ResolutionChoiceKind::ChooseTarget { options, optional: false, effect: PendingEffect::AttachTargetToPendingTrigger, .. },
            }) if *player == controller && *s == source
                && matches!(options.as_slice(), [Target::Object(ObjectId(1)), Target::Object(ObjectId(2))])));
        }
    }
    std::mem::forget((registry, state));
}

/// The answer to a trigger's target prompt (CR 603.3d): the trigger
/// waiting at the front of its controller's queue (the other queue empty,
/// as the active player's empties before the other's is begun, 603.3b)
/// goes on the stack holding the target chosen, and nothing else waits.
/// For a trigger either player's on either player's turn, and either of
/// the two creatures chosen.
fn attach((controller, active): (PlayerId, PlayerId)) {
    // SAFETY: as in `requirement`.
    unsafe { *std::ptr::addr_of_mut!(ALLOWED) = None };
    let (registry, mut state, trigger, source) = board(controller, active);
    if controller == active {
        state.pending_trigger_pushes_ap.push(trigger);
    } else {
        state.pending_trigger_pushes_nap.push(trigger);
    }
    let chosen = Target::Object(ObjectId(1 + u64::from(kani::any::<bool>())));

    crate::engine::apply_pending_effect(&mut state, &chosen, &PendingEffect::AttachTargetToPendingTrigger, &registry);

    assert!(state.pending_trigger_pushes_ap.is_empty() && state.pending_trigger_pushes_nap.is_empty());
    assert!(matches!(state.stack.as_slice(), [StackEntry::Trigger(t)]
        if t.source.id == source && t.source.controller == controller && t.source.chosen_targets.as_slice() == [chosen.clone()]));
    std::mem::forget((registry, state));
}

/// The proofs, the requirement and the targets allowed standing in as
/// above. Not a card proof's stand-ins: those record the pending effect a
/// card applies rather than apply it, and the answer here is one.
macro_rules! each_push {
    ($($name:ident: $proof:ident($case:expr),)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::triggers::target_requirement, requirement)]
        #[kani::stub(crate::engine::valid_targets_for_req, allowed)]
        /// The proof above it, for one case.
        fn $name() {
            $proof($case);
        }
    )*};
}

each_push! {
    a_trigger_that_does_not_target_goes_on_the_stack_as_it_is: pushing((None, PlayerId(0), PlayerId(0))),
    a_trigger_with_no_target_allowed_is_removed: pushing((Some(0), PlayerId(0), PlayerId(1))),
    a_trigger_with_one_target_allowed_goes_on_the_stack_holding_it: pushing((Some(1), PlayerId(1), PlayerId(0))),
    a_trigger_of_the_active_players_with_two_targets_allowed_asks_which: pushing((Some(2), PlayerId(0), PlayerId(0))),
    a_trigger_of_the_other_players_with_two_targets_allowed_asks_which: pushing((Some(2), PlayerId(1), PlayerId(0))),
    the_active_players_trigger_goes_on_the_stack_holding_the_target_chosen: attach((PlayerId(1), PlayerId(1))),
    the_other_players_trigger_goes_on_the_stack_holding_the_target_chosen: attach((PlayerId(0), PlayerId(1))),
}

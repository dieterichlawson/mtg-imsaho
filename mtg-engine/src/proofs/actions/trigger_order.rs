//! The order in which abilities that triggered together go on the stack
//! (`triggers::process_pending_trigger_pushes`, CR 603.3b): the active
//! player's first, then the other's, each player's in the order they
//! choose, asked of them when it matters. What going on the stack does to
//! one trigger is `trigger_targets`' (`push_one_pending_trigger`), recorded
//! here; what an ordering question says of each trigger (its name, its
//! source's power and toughness) is display, stood in for as nothing.

use crate::proofs::held::Held;
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, Blank};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::triggers::{PendingTrigger, TriggerEvent};
use crate::types::Zone;

/// The triggers put on the stack, in order: each one's source and whether
/// it had been ordered by its controller already.
static mut PUSHED: Held<[Option<(ObjectId, bool)>; 3]> = Held::new([None; 3], (&raw const PUSHED).cast());
static mut PUSHED_N: Held<usize> = Held::new(0, (&raw const PUSHED_N).cast());

/// Stands in for `triggers::push_one_pending_trigger`: recorded.
#[allow(dead_code)] // each named only by `kani::stub`
fn pushed(_: &mut GameState, trigger: PendingTrigger, _: &CardRegistry) {
    // SAFETY: a proof runs on one thread, and only this function writes PUSHED and PUSHED_N.
    let (p, n) = unsafe { (&mut *std::ptr::addr_of_mut!(PUSHED.value), &mut *std::ptr::addr_of_mut!(PUSHED_N.value)) };
    assert!(*n < 3, "no more triggers than this proof queues");
    p[*n] = Some((trigger.source.id, trigger.ordered));
    *n += 1;
    std::mem::forget(trigger);
}
#[allow(dead_code)]
fn no_name(_: &PendingTrigger, _: &CardRegistry, _: Option<&GameState>) -> String {
    String::new()
}
#[allow(dead_code)]
fn no_details(t: &PendingTrigger, _: &CardRegistry, _: &GameState) -> crate::state::TriggerOrderOption {
    crate::state::TriggerOrderOption {
        source: t.source.id, source_name: String::new(), power_toughness: None,
        kind: String::new(), ability: String::new(), cause: String::new(),
    }
}
#[allow(dead_code)]
fn no_tag(_: &GameState, _: &CardRegistry, _: ObjectId) -> String {
    String::new()
}

/// What each case queues: (source index 0-2, description, whether already
/// ordered) for the active player's queue and for the other's. Sources 0
/// and 1 are the active player's (objects 1 and 2), 2 the other's (3).
type Queued = ([Option<(usize, &'static str, bool)>; 2], [Option<(usize, &'static str, bool)>; 2]);

/// A board of three permanents and the queues `case` names, built as the
/// collector files them, the first player active.
fn queues(case: &Queued) -> (CardRegistry, GameState, [ObjectId; 3]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    state.active_player = PlayerId(0);
    let ids = [
        state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1)),
        state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1)),
        state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(1), Some(1)),
    ];
    state.pending_trigger_pushes_ap.reserve(2);
    state.pending_trigger_pushes_nap.reserve(2);
    let mut c = crate::triggers::collect::Collector::new(PlayerId(0));
    for &(source, description, _) in case.0.iter().chain(&case.1).flatten() {
        let controller = if source == 2 { PlayerId(1) } else { PlayerId(0) };
        c.emit(ids[source], blank, controller, description.into(), TriggerEvent::Upkeep);
    }
    for (t, &(_, _, ordered)) in c.ap.iter_mut().zip(case.0.iter().flatten()) {
        t.ordered = ordered;
    }
    for (t, &(_, _, ordered)) in c.nap.iter_mut().zip(case.1.iter().flatten()) {
        t.ordered = ordered;
    }
    for t in c.ap.drain(..) {
        state.pending_trigger_pushes_ap.push(t);
    }
    for t in c.nap.drain(..) {
        state.pending_trigger_pushes_nap.push(t);
    }
    std::mem::forget(c);
    (registry, state, ids)
}

/// The cases, by number: 0, the active player's two triggers of different
/// sources; 1, two of one source and one ability; 2, one of the active
/// player's and one of the other's; 3, two of the active player's already
/// ordered; 4, the other player's two of different abilities of one
/// source, with nothing of the active player's.
const CASES: [Queued; 5] = [
    ([Some((0, "a", false)), Some((1, "b", false))], [None, None]),
    ([Some((0, "a", false)), Some((0, "a", false))], [None, None]),
    ([Some((0, "a", false)), None], [Some((2, "c", false)), None]),
    ([Some((1, "b", true)), Some((0, "a", true))], [None, None]),
    ([None, None], [Some((2, "c", false)), Some((2, "d", false))]),
];

/// Abilities that triggered together put on the stack (CR 603.3b, 101.4):
/// the active player's first, then the other player's; a player's own
/// triggers in the order they choose, so with two or more that can be
/// told apart (of different sources, or different abilities of one), and
/// not yet ordered, they are asked the order, of exactly those, nothing
/// going on the stack until they answer; with only interchangeable ones,
/// or ones they have ordered already, each goes on front first, unasked.
/// For each of the cases [`CASES`] names.
fn processed(k: usize) {
    let case = &CASES[k];
    let (registry, mut state, ids) = queues(case);

    crate::triggers::process_pending_trigger_pushes(&mut state, &registry);

    // SAFETY: as in `pushed`.
    let (p, n) = unsafe { (*std::ptr::addr_of!(PUSHED.value), *std::ptr::addr_of!(PUSHED_N.value)) };
    match k {
        0 | 4 => {
            let (player, ap) = if k == 0 { (PlayerId(0), true) } else { (PlayerId(1), false) };
            assert!(n == 0);
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player: p, choice: ResolutionChoiceKind::ChooseTriggerOrder { ap_queue, indices, options, .. }, ..
            }) if *p == player && *ap_queue == ap && indices.as_slice() == [0, 1] && options.len() == 2));
            let queued = if ap { &state.pending_trigger_pushes_ap } else { &state.pending_trigger_pushes_nap };
            assert!(queued.len() == 2);
        }
        _ => {
            let expected: [Option<(ObjectId, bool)>; 3] = match k {
                1 => [Some((ids[0], false)), Some((ids[0], false)), None],
                2 => [Some((ids[0], false)), Some((ids[2], false)), None],
                _ => [Some((ids[1], true)), Some((ids[0], true)), None],
            };
            assert!(n == 2 && p == expected && state.awaiting_action.is_none());
            assert!(state.pending_trigger_pushes_ap.is_empty() && state.pending_trigger_pushes_nap.is_empty());
        }
    }
    std::mem::forget((registry, state));
}

/// The proofs of the triggers put on the stack, one per case.
macro_rules! each_queue {
    ($($name:ident: $case:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::triggers::push_one_pending_trigger, pushed)]
            #[kani::stub(crate::triggers::PendingTrigger::display_name_with_state, no_name)]
            #[kani::stub(crate::triggers::PendingTrigger::order_option, no_details)]
            #[kani::stub(crate::engine::cards_flow::source_tag, no_tag)]
            /// [`processed`] for the case of its name.
            fn $name() {
                processed($case);
            }
        }
    )*};
}

each_queue! {
    two_triggers_of_the_active_player_from_two_sources_are_asked_their_order: 0;
    two_triggers_of_one_ability_of_one_source_go_on_unasked: 1;
    the_active_players_trigger_goes_on_before_the_other_players: 2;
    triggers_already_ordered_go_on_in_their_order_unasked: 3;
    two_triggers_of_the_other_player_of_two_abilities_are_asked_their_order: 4;
}

/// The active player's queue as `process_pending_trigger_pushes` found it
/// after an ordering answer, each trigger's source and whether it had been
/// ordered, and how many times it was called.
static mut QUEUED: Held<[Option<(ObjectId, bool)>; 2]> = Held::new([None; 2], (&raw const QUEUED).cast());
static mut PROCESSED: Held<usize> = Held::new(0, (&raw const PROCESSED).cast());

/// Stands in for `triggers::process_pending_trigger_pushes`, proved above:
/// the queue recorded.
#[allow(dead_code)] // each named only by `kani::stub`
fn processed_after(state: &mut GameState, _: &CardRegistry) {
    let q = &state.pending_trigger_pushes_ap;
    assert!(q.len() <= 2, "no more triggers than this proof queues");
    // SAFETY: as in `pushed`, for QUEUED and PROCESSED.
    unsafe {
        *std::ptr::addr_of_mut!(QUEUED.value) = [q.first().map(|t| (t.source.id, t.ordered)), q.get(1).map(|t| (t.source.id, t.ordered))];
        *std::ptr::addr_of_mut!(PROCESSED.value) += 1;
    }
}
#[allow(dead_code)]
fn no_log_name(_: &PendingTrigger, _: &CardRegistry, _: &GameState) -> String {
    String::new()
}

/// The order of two triggers of the active player's answered (CR 603.3b,
/// `resolve_choice` of `ChooseTriggerOrder`): with the whole order, each
/// trigger offered exactly once, they are put back at the front of the
/// queue in that order, each marked as ordered (so none is asked about
/// again), and the queue goes on; with one of them, it goes on the stack
/// now, and the queue goes on with the rest, which asks again if it must;
/// anything else (an order naming one twice or leaving one out, one past
/// those offered) is refused, the question standing as it was asked and
/// the queue as it was. The reply each harness's: 0 and 1 the whole order
/// either way round, 2 one twice, 3 one left out, 4 and 5 one of them, 6
/// one past them.
fn order_answered(reply: u8) {
    let (registry, mut state, ids) = queues(&CASES[0]);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(0),
        source: ids[0],
        choice: ResolutionChoiceKind::ChooseTriggerOrder {
            description: String::new(),
            options: vec![String::new(), String::new()],
            ap_queue: true,
            indices: vec![0, 1],
            details: Vec::new(),
        },
    });
    let answer = match reply {
        0 => crate::actions::ResolvedChoice::ChosenOrder(vec![0, 1]),
        1 => crate::actions::ResolvedChoice::ChosenOrder(vec![1, 0]),
        2 => crate::actions::ResolvedChoice::ChosenOrder(vec![1, 1]),
        3 => crate::actions::ResolvedChoice::ChosenOrder(vec![0]),
        k => crate::actions::ResolvedChoice::ChosenIndex(usize::from(k - 4), String::new()),
    };

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &answer, &registry);

    // SAFETY: as in `pushed`.
    let (queued, processed, p, n) = unsafe { (
        *std::ptr::addr_of!(QUEUED.value), *std::ptr::addr_of!(PROCESSED.value),
        *std::ptr::addr_of!(PUSHED.value), *std::ptr::addr_of!(PUSHED_N.value),
    ) };
    match reply {
        0 | 1 => {
            let (a, b) = if reply == 0 { (ids[0], ids[1]) } else { (ids[1], ids[0]) };
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(n == 0 && processed == 1 && queued == [Some((a, true)), Some((b, true))]);
        }
        4 | 5 => {
            let (chosen, rest) = if reply == 4 { (ids[0], ids[1]) } else { (ids[1], ids[0]) };
            assert!(matches!(applied, crate::engine::Applied::Continue));
            assert!(n == 1 && p[0] == Some((chosen, false)));
            assert!(processed == 1 && queued == [Some((rest, false)), None]);
        }
        _ => {
            assert!(matches!(applied, crate::engine::Applied::ReturnNow));
            assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player: PlayerId(0),
                choice: ResolutionChoiceKind::ChooseTriggerOrder { ap_queue: true, indices, .. }, ..
            }) if indices.as_slice() == [0, 1]));
            let q = &state.pending_trigger_pushes_ap;
            assert!(n == 0 && processed == 0 && q.len() == 2);
            assert!(q[0].source.id == ids[0] && !q[0].ordered && q[1].source.id == ids[1] && !q[1].ordered);
        }
    }
    std::mem::forget((registry, state, answer));
}

/// The proofs of an ordering answered, one per reply.
macro_rules! each_order_answer {
    ($($name:ident: $reply:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::triggers::push_one_pending_trigger, pushed)]
            #[kani::stub(crate::triggers::process_pending_trigger_pushes, processed_after)]
            #[kani::stub(crate::triggers::PendingTrigger::log_name, no_log_name)]
            /// [`order_answered`] for the reply of its name.
            fn $name() {
                order_answered($reply);
            }
        }
    )*};
}

each_order_answer! {
    the_order_given_is_the_order_the_triggers_go_on: 0;
    the_order_given_reversed_is_the_order_the_triggers_go_on: 1;
    an_order_naming_a_trigger_twice_is_refused: 2;
    an_order_leaving_a_trigger_out_is_refused: 3;
    the_first_trigger_chosen_goes_on_now: 4;
    the_second_trigger_chosen_goes_on_now: 5;
    a_trigger_past_those_offered_is_refused: 6;
}

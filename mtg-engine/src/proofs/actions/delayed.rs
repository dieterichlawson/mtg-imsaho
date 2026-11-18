//! The delayed triggered ability a card proof sees Geist of Saint Traft
//! leave: "Exile that token at end of combat" (CR 603.7), an entry in
//! `end_of_combat_exiles`. Here, what the engine then does with it: the
//! end of combat step's beginning triggers it, once (603.7a), and its
//! resolving exiles the token if the token is still on the battlefield: a
//! token that has left is no longer that object (400.7), and nothing is
//! exiled.

use crate::events::GameEvent;
use crate::ids::PlayerId;
use crate::proofs::cards::{acted, asked_for, blank_board, case_by_case, Act};
use crate::state::{EndOfCombatExileEntry, StackEntry};
use crate::triggers::TriggerEvent;
use crate::types::{Step, Zone};

/// With a delayed exile of a token made this combat, the end of combat
/// step beginning (CR 507.1, 603.7a) puts exactly one triggered ability on
/// the stack, the token's exile, with Geist of Saint Traft as its source
/// and controlled by the player whose ability made it, and leaves no
/// delayed exile behind; nothing else is done. For the ability's
/// controller, the active player and whether the token is on the
/// battlefield or already in exile, each a constant of each harness: left
/// to the solver, which of the collector's two lists the ability was filed
/// in was a question symbolic execution could not settle, nor how many
/// permanents the step's own triggers were looked for on, and the queue of
/// abilities read back was of a length it could not see (it built the
/// prompt to order a queue of one).
fn end_of_combat((controller, active, there): (PlayerId, PlayerId, bool)) {
    let (registry, mut state, blank) = blank_board();
    state.active_player = active;
    let geist = state.create_object(blank, controller, Zone::Battlefield, Some(2), Some(2));
    let token = state.create_object(blank, controller, if there { Zone::Battlefield } else { Zone::Exile }, Some(4), Some(4));
    state.get_object_mut(token).expect("just created").is_token = true;
    state.end_of_combat_exiles.push(EndOfCombatExileEntry {
        target_id: token, source_id: geist, source_card_id: blank, controller,
        description: "exile the Angel token".into(),
    });
    state.events.push(GameEvent::StepStarted { step: Step::EndCombat });
    // Room for the trigger on its way to the stack, as `new_game` gives
    // the stack and the events theirs.
    state.pending_trigger_pushes_ap.reserve(1);
    state.pending_trigger_pushes_nap.reserve(1);
    state.pending_triggers.reserve(1);

    assert!(crate::triggers::collect_triggers(&mut state, &registry));
    assert!(state.end_of_combat_exiles.is_empty() && state.awaiting_action.is_none());
    assert!(matches!(state.stack.as_slice(), [StackEntry::Trigger(t)]
        if matches!(t.event, TriggerEvent::DelayedTokenExile { target_id } if target_id == token)
            && t.source.controller == controller && t.source.id == geist));
    assert!(acted() == 0);
    std::mem::forget((registry, state));
}

case_by_case! { end_of_combat;
    the_end_of_combat_puts_the_tokens_exile_on_the_stack: (PlayerId(0), PlayerId(0), true),
    the_end_of_combat_puts_the_tokens_exile_on_the_stack_on_the_other_players_turn: (PlayerId(0), PlayerId(1), true),
    the_end_of_combat_puts_the_other_players_tokens_exile_on_the_stack: (PlayerId(1), PlayerId(1), true),
    the_end_of_combat_puts_the_other_players_tokens_exile_on_the_stack_on_the_first_players_turn: (PlayerId(1), PlayerId(0), true),
    the_end_of_combat_puts_a_gone_tokens_exile_on_the_stack: (PlayerId(0), PlayerId(0), false),
    the_end_of_combat_puts_a_gone_tokens_exile_on_the_stack_on_the_other_players_turn: (PlayerId(0), PlayerId(1), false),
    the_end_of_combat_puts_the_other_players_gone_tokens_exile_on_the_stack: (PlayerId(1), PlayerId(1), false),
    the_end_of_combat_puts_the_other_players_gone_tokens_exile_on_the_stack_on_the_first_players_turn: (PlayerId(1), PlayerId(0), false),
}

/// The token's exile resolving (CR 608.2) moves the token to exile when it
/// is on the battlefield, and does nothing when it has left (400.7); it
/// leaves the stack either way. For the token on the battlefield or
/// already in exile, its source on the battlefield or gone (603.7d: the
/// ability resolves without it), and the ability's controller a constant
/// of each harness, as for [`end_of_combat`].
fn exile_resolving(controller: PlayerId) {
    let (registry, mut state, blank) = blank_board();
    let geist = state.create_object(blank, controller, if kani::any() { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
    let there: bool = kani::any();
    let token = state.create_object(blank, controller, if there { Zone::Battlefield } else { Zone::Exile }, Some(4), Some(4));
    state.get_object_mut(token).expect("just created").is_token = true;
    // Built as the end of combat builds it, through the collector.
    let mut c = crate::triggers::collect::Collector::new(controller);
    c.emit(geist, blank, controller, "exile the Angel token".into(), TriggerEvent::DelayedTokenExile { target_id: token });
    state.stack.push(StackEntry::Trigger(c.ap.pop().expect("just emitted")));

    assert!(crate::triggers::resolve_next_trigger(&mut state, &registry));
    assert!(state.stack.is_empty());
    if there {
        assert!(asked_for(&[Act::Move(token, Zone::Exile)]));
    } else {
        assert!(acted() == 0);
    }
    std::mem::forget((registry, state));
}

case_by_case! { exile_resolving;
    the_tokens_exile_resolving_exiles_the_token_if_it_is_there: PlayerId(0),
    the_other_players_tokens_exile_resolving_exiles_the_token_if_it_is_there: PlayerId(1),
}

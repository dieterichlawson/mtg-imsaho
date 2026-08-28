//! CR 603.2: a condition on the triggering event is read when the event
//! happens, not when the ability resolves.
//!
//! Mentor of the Meek — "whenever another creature with power 2 or less enters
//! under your control" — checked the power inside its resolution handler. That
//! is wrong in both directions, and the wrongness is symmetric:
//!
//! - a creature that entered with power 2 and was pumped before the trigger
//!   resolved DID trigger, but the handler saw the inflated power and silently
//!   did nothing;
//! - a creature that entered with power 3 and was shrunk before resolution did
//!   NOT trigger, but the handler saw the reduced power and offered the pay-{1}
//!   prompt anyway.
//!
//! Reading the condition at dispatch fixes both, and stops a trigger that
//! shouldn't exist from taking up a priority window on the way.
//!
//! The second half of the file is the other end of the same card: what
//! happens once the trigger resolves and the "you may pay {1}" is answered.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::GameState;
use mtg_engine::triggers::{self, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;
/// Put a creature onto the battlefield through `move_object`, so the entering
/// event is emitted the way it is in a real game.
fn enter_creature(state: &mut GameState, reg: &CardRegistry, name: &str, owner: mtg_engine::ids::PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown {name}"));
    let data = reg.card_data(card_id).unwrap();
    let id = state.create_object(card_id, owner, Zone::Hand, data.power, data.toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    state.move_object(id, Zone::Battlefield, reg);
    id
}

fn enter_watch_triggers(state: &GameState, watcher: ObjectId) -> usize {
    state.stack.iter()
        .filter(|e| matches!(e, mtg_engine::state::StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: watcher_id, .. },
                event: TriggerEvent::CreatureEntered { .. } }) if *watcher_id == watcher))
        .count()
}

#[test]
fn mentor_triggers_for_a_small_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    // Avacyn's Pilgrim is a 1/1.
    enter_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    triggers::collect_triggers(&mut state, &reg);

    assert_eq!(enter_watch_triggers(&state, mentor), 1,
        "a 1/1 entering under your control satisfies 'power 2 or less'");
}

#[test]
fn mentor_does_not_trigger_for_a_big_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    // Bloodgift Demon is a 5/4.
    enter_creature(&mut state, &reg, "Bloodgift Demon", P0);
    triggers::collect_triggers(&mut state, &reg);

    assert_eq!(enter_watch_triggers(&state, mentor), 0,
        "a 5/4 does not satisfy 'power 2 or less', so the ability must not \
         trigger at all — not trigger and then decline to do anything");
}

#[test]
fn mentor_does_not_trigger_for_an_opponents_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    enter_creature(&mut state, &reg, "Avacyn's Pilgrim", P1);
    triggers::collect_triggers(&mut state, &reg);

    assert_eq!(enter_watch_triggers(&state, mentor), 0,
        "'under your control' excludes an opponent's creature");
}

#[test]
fn mentor_does_not_trigger_for_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mentor = enter_creature(&mut state, &reg, "Mentor of the Meek", P0);
    triggers::collect_triggers(&mut state, &reg);

    assert_eq!(enter_watch_triggers(&state, mentor), 0,
        "'another creature' excludes the Mentor itself");
}

/// The decision is locked in at entry: pumping the creature afterwards cannot
/// un-trigger an ability that already triggered.
#[test]
fn pumping_after_entry_does_not_undo_the_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    let pilgrim = enter_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(enter_watch_triggers(&state, mentor), 1, "test precondition");

    // A pump spell resolves in response, taking it to 4/4.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ModifyPT {
        target: pilgrim, power_mod: 3, toughness_mod: 3,
    });
    assert_eq!(state.effective_power(pilgrim, &reg), Some(4), "test precondition");

    assert_eq!(enter_watch_triggers(&state, mentor), 1,
        "the trigger already fired on a power-1 creature; pumping it does not \
         remove the ability from the stack (CR 603.2)");
}

/// And the mirror: shrinking a big creature after it entered cannot conjure a
/// trigger that never happened.
#[test]
fn shrinking_after_entry_does_not_create_a_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    let demon = enter_creature(&mut state, &reg, "Bloodgift Demon", P0);
    triggers::collect_triggers(&mut state, &reg);

    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ModifyPT {
        target: demon, power_mod: -4, toughness_mod: 0,
    });
    assert_eq!(state.effective_power(demon, &reg), Some(1), "test precondition");

    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(enter_watch_triggers(&state, mentor), 0,
        "it entered as a 5/4 and never triggered; shrinking it afterwards \
         cannot retroactively satisfy the condition");
}

// -------------------------------------------------------------------------
// "...you may pay {1}. If you do, draw a card."
// -------------------------------------------------------------------------

/// Paying may involve tapping lands for the mana (CR 601.2g via 608.2g). The
/// card used to walk the mana pool by hand and spend a floating unit if it
/// found one, so with an empty pool and untapped Plains, saying "yes" paid
/// nothing and drew nothing.
#[test]
fn mentor_taps_lands_to_pay_its_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);
    let plains = named_permanent(&mut state, &reg, "Plains", P0);
    stock_library(&mut state, &reg, P0, 3);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "test setup: nothing floating");

    let small = ready_creature(&mut state, P0, 1, 1);
    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small, P0, &reg);
    behavior.on_yes_no_choice(&mut state, mentor, true, &reg);

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 1,
        "the {{1}} is paid by tapping a land, and the card is drawn");
    assert!(state.get_object(plains).unwrap().tapped, "which is where the mana came from");
}

/// Ruling 2025-01-24: "While resolving the triggered ability of Mentor of the
/// Meek, you can't pay {1} multiple times to draw more than one card." One
/// question, one answer, one card.
#[test]
fn mentor_draws_one_card_however_much_mana_is_available() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);
    for _ in 0..5 {
        named_permanent(&mut state, &reg, "Plains", P0);
    }
    stock_library(&mut state, &reg, P0, 5);

    let small = ready_creature(&mut state, P0, 1, 1);
    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small, P0, &reg);

    // Answered through the engine, so the prompt is consumed the way it is in
    // a real game and anything asked again would still be sitting there.
    let state = mtg_engine::engine::submit_action(&state, &mtg_engine::actions::Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(true),
    }, &reg);

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 1,
        "one card, not one per available mana");
    assert!(state.awaiting_action.is_none(), "and it does not ask again");
}

/// "You **may** pay" — declining is an answer, and it costs nothing.
#[test]
fn mentor_declining_costs_nothing_and_draws_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);
    let plains = named_permanent(&mut state, &reg, "Plains", P0);
    stock_library(&mut state, &reg, P0, 3);

    let small = ready_creature(&mut state, P0, 1, 1);
    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small, P0, &reg);
    behavior.on_yes_no_choice(&mut state, mentor, false, &reg);

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 0, "no draw");
    assert!(!state.get_object(plains).unwrap().tapped, "and nothing was tapped for it");
}

/// Saying yes with nothing to pay with draws nothing — "if you do" was not
/// satisfied — and does not leave the game in a half-paid state.
#[test]
fn mentor_saying_yes_with_no_mana_available_draws_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);
    stock_library(&mut state, &reg, P0, 3);

    let small = ready_creature(&mut state, P0, 1, 1);
    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small, P0, &reg);
    behavior.on_yes_no_choice(&mut state, mentor, true, &reg);

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 0,
        "no mana anywhere, so the cost is unpaid and the draw does not happen");
}

//! Regression test for Bug C (`BUG_REPORT_8SEAT.md)`: LTB triggers were
//! displaying controller as PlayerId(255) because the collector had no
//! way to recover the last controller of a permanent after it had moved
//! off the battlefield. Per CR 603.10c, the controller must be whoever
//! controlled the permanent immediately before it left the battlefield.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent};
use mtg_engine::types::{Step, Zone};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

#[test]
fn fiend_hunter_ltb_trigger_has_correct_controller() {
    // Fiend Hunter is controlled by p0, then leaves the battlefield.
    // Its LTB trigger's controller must be p0, not PlayerId(255).
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hunter = named_permanent(&mut state, &registry, "Fiend Hunter", P0);

    state.move_object(hunter, Zone::Graveyard, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let ltb = state.stack.iter().find_map(|e| match e {
        StackEntry::Trigger(t @ PendingTrigger { event: TriggerEvent::LeftBattlefield, .. }) => Some(t),
        _ => None,
    }).expect("Fiend Hunter's LTB trigger should be on the stack");

    assert_eq!(ltb.controller(), P0,
        "LTB trigger controller must be p0 (last controller of Fiend Hunter), not {:?}",
        ltb.controller());
    assert_ne!(ltb.controller(), PlayerId(255),
        "LTB trigger controller must not be the sentinel p255");
}

#[test]
fn fiend_hunter_controlled_by_p1_has_p1_controller() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hunter = named_permanent(&mut state, &registry, "Fiend Hunter", P1);

    state.move_object(hunter, Zone::Graveyard, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let ltb = state.stack.iter().find_map(|e| match e {
        StackEntry::Trigger(t @ PendingTrigger { event: TriggerEvent::LeftBattlefield, .. }) => Some(t),
        _ => None,
    }).expect("Fiend Hunter's LTB trigger should be on the stack");

    assert_eq!(ltb.controller(), P1, "LTB trigger must carry p1 as last controller");
}

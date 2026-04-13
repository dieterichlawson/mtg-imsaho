//! Bug: Grimoire of the Dead does not remove study counters as an activation cost.
//!
//! Oracle text: "{T}, Remove three study counters from Grimoire of the Dead
//! and sacrifice it: Put all creature cards from all graveyards onto the
//! battlefield under your control. They're black Zombies in addition to their
//! other colors and types."
//!
//! "Remove three study counters" is an activation COST (before the colon).
//! The ActivatedAbilityDef struct (cards/mod.rs:97-110) has no
//! counter_removal_cost field, so the engine cannot pay this cost. The card
//! checks `study_counters >= 3` as a guard in activated_abilities() but never
//! actually removes counters. The comment at grimoire_of_the_dead.rs:131-135
//! acknowledges this: "counter removal is moot" because the Grimoire is
//! already sacrificed (which clears all counters via move_object).
//!
//! This test calls on_activate_ability directly (bypassing the engine's
//! sacrifice cost payment) to keep the Grimoire on the battlefield and observe
//! that the 3 study counters are never removed.

mod common;

use common::{game_at_step, ready_creature, P0};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

#[test]
#[ignore] // Pipeline bug — awaiting fix
fn bug_grimoire_study_counters_not_removed_as_activation_cost() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Grimoire of the Dead on the battlefield with 5 study counters.
    // Using 5 (not 3) so we can distinguish "removed 3, leaving 2" from
    // "removed none, still 5" or "cleared all, leaving 0".
    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    state.add_counters(grimoire, CounterType::Study, 5);
    assert_eq!(
        state.get_counter_count(grimoire, CounterType::Study), 5,
        "precondition: Grimoire should start with 5 study counters"
    );

    // Place a creature in the graveyard for the ability effect to return.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().name = "Test Creature".into();
    state.move_object(creature, Zone::Graveyard, &reg);

    // Call on_activate_ability directly, bypassing the engine's cost payment
    // (which taps and sacrifices the Grimoire). This keeps the Grimoire on the
    // battlefield so we can observe counter state after the ability fires.
    // The Oracle cost "Remove three study counters" should be reflected here.
    let behavior = reg.get(card_id).unwrap();
    behavior.on_activate_ability(&mut state, grimoire, 1, &[], &reg);

    // Oracle: "Remove three study counters from Grimoire of the Dead" is a cost.
    // Starting from 5 study counters, removing 3 should leave 2.
    let remaining = state.get_counter_count(grimoire, CounterType::Study);
    assert_eq!(
        remaining, 2,
        "Oracle text specifies 'Remove three study counters' as an activation cost. \
         Started with 5 study counters; expected 2 remaining after removing 3, \
         but got {}. Neither the engine (ActivatedAbilityDef has no counter_removal_cost) \
         nor on_activate_ability removes the counters (grimoire_of_the_dead.rs:131-135).",
        remaining,
    );
}

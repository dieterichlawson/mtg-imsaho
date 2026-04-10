/// Regression tests for Bug A (BUG_REPORT_8SEAT.md): cards without a
/// SelfDies or LeavesBattlefield TriggeredAbilityDef were pushing empty
/// triggers onto the stack whenever they left the battlefield, polluting
/// the LLM prompt with no-op [RESPOND TO ...] cycles.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::{Step, Zone};

const P0: PlayerId = PlayerId(0);

/// Helper: kill a battlefield creature via lethal damage + SBA.
fn kill_creature(state: &mut mtg_engine::state::GameState, registry: &CardRegistry, id: mtg_engine::ids::ObjectId) {
    state.get_object_mut(id).unwrap().damage_marked = 99;
    mtg_engine::sba::check_state_based_actions(state, registry);
}

#[test]
fn typhoid_rats_death_creates_no_stack_entries() {
    // Oracle: Typhoid Rats has only "Deathtouch". No dies or LTB handler.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let rats = named_creature(&mut state, &registry, "Typhoid Rats", P0);

    kill_creature(&mut state, &registry, rats);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let trigger_count = state.stack.iter().filter(|e| matches!(e, StackEntry::Trigger(_))).count();
    assert_eq!(trigger_count, 0,
        "Typhoid Rats has no dies/LTB handler per oracle; stack should have no triggers");
}

#[test]
fn fortress_crab_death_creates_no_stack_entries() {
    // Oracle: Fortress Crab is a vanilla 1/6.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let crab = named_creature(&mut state, &registry, "Fortress Crab", P0);

    kill_creature(&mut state, &registry, crab);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let trigger_count = state.stack.iter().filter(|e| matches!(e, StackEntry::Trigger(_))).count();
    assert_eq!(trigger_count, 0, "Fortress Crab is vanilla — no triggers");
}

#[test]
fn walking_corpse_death_creates_no_stack_entries() {
    // Oracle: Walking Corpse is vanilla.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wc = named_creature(&mut state, &registry, "Walking Corpse", P0);

    kill_creature(&mut state, &registry, wc);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let trigger_count = state.stack.iter().filter(|e| matches!(e, StackEntry::Trigger(_))).count();
    assert_eq!(trigger_count, 0, "Walking Corpse is vanilla — no triggers");
}

#[test]
fn aura_leaving_battlefield_creates_no_ltb_trigger() {
    // Oracle: Dead Weight is an aura with no LTB text. Moving it off the
    // battlefield should not push an LTB trigger.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let target = named_creature(&mut state, &registry, "Grizzly Bears", P0);
    let aura = named_creature(&mut state, &registry, "Dead Weight", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(target);

    // Move aura off battlefield (simulates destruction).
    state.move_object(aura, Zone::Graveyard, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let ltb_count = state.stack.iter().filter(|e|
        matches!(e, StackEntry::Trigger(PendingTrigger::LeftBattlefield { .. }))
    ).count();
    assert_eq!(ltb_count, 0,
        "Dead Weight has no LTB text per oracle; LTB trigger should not be created");
}

#[test]
fn fiend_hunter_ltb_trigger_still_fires() {
    // Oracle: "When this creature leaves the battlefield, return the exiled
    // card to the battlefield under its owner's control."
    // Regression in the opposite direction: the gate must not break this.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hunter = named_creature(&mut state, &registry, "Fiend Hunter", P0);

    state.move_object(hunter, Zone::Graveyard, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let has_ltb = state.stack.iter().any(|e|
        matches!(e, StackEntry::Trigger(PendingTrigger::LeftBattlefield { .. }))
    );
    assert!(has_ltb, "Fiend Hunter's LTB trigger must still fire");
}

#[test]
fn doomed_traveler_selfdies_trigger_still_fires() {
    // Oracle: "When this creature dies, create a 1/1 white Spirit creature
    // token with flying." Gate must not break this.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let traveler = named_creature(&mut state, &registry, "Doomed Traveler", P0);

    kill_creature(&mut state, &registry, traveler);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let has_dies = state.stack.iter().any(|e|
        matches!(e, StackEntry::Trigger(PendingTrigger::SelfDies { .. }))
    );
    assert!(has_dies, "Doomed Traveler's dies trigger must still fire");
}

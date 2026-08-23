//! Regression tests for the Evil Twin "enters as a copy" SBA guard (CR 614.1d).
//!
//! Evil Twin briefly exists as its printed 0/0 while the copy choice is
//! pending. The transient `entering_copy_source` flag — armed at entry by
//! move_object, cleared when the choice concludes — protects it during that
//! window and ONLY that window. Previously the guard was the static
//! `enters_as_copy()` card property, which left every resolved / declined /
//! no-target Evil Twin permanently immune to state-based-action death.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::PendingEffect;
use mtg_engine::types::*;

fn reg() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Enter an Evil Twin through the real entry chokepoint (move_object).
fn enter_twin(state: &mut mtg_engine::state::GameState, r: &CardRegistry) -> mtg_engine::ids::ObjectId {
    let card = r.get_id_by_name("Evil Twin").unwrap();
    let twin = state.create_object(card, P0, Zone::Hand, Some(0), Some(0));
    state.get_object_mut(twin).unwrap().name = "Evil Twin".into();
    state.move_object(twin, Zone::Battlefield, r);
    twin
}

/// The guard is armed at entry, so SBA doesn't kill the 0/0 before the copy
/// choice resolves.
#[test]
fn guard_armed_at_entry_protects_before_copy_resolves() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let twin = enter_twin(&mut state, &r);

    assert!(state.get_object(twin).unwrap().entering_copy_source,
        "move_object must arm the copy-guard at entry");
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Battlefield,
        "the 0/0 must survive SBA while the copy choice is pending");
}

/// After copying, the guard is disarmed and the permanent is once again
/// subject to SBA death.
#[test]
fn copy_success_disarms_guard_and_is_mortal_again() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bears = named_creature(&mut state, &r, "Grizzly Bears", P1);
    let twin = enter_twin(&mut state, &r);

    // Resolve the copy onto Grizzly Bears.
    engine::apply_pending_effect(
        &mut state, &Target::Object(bears),
        &PendingEffect::CopyCreature { source_id: twin }, &r,
    );
    assert!(!state.get_object(twin).unwrap().entering_copy_source,
        "copy resolution must disarm the guard");

    // It's now a 2/2 — lethal damage plus SBA must kill it.
    state.get_object_mut(twin).unwrap().damage_marked = 5;
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "a resolved Evil Twin must die to lethal damage like any creature");
}

/// Declining the copy disarms the guard, so the printed 0/0 dies to SBA.
#[test]
fn declining_copy_lets_the_0_0_die() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _other = named_creature(&mut state, &r, "Grizzly Bears", P1);
    let twin = enter_twin(&mut state, &r);

    // Present the copy choice, then decline it.
    let behavior = r.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &r);
    assert!(state.awaiting_action.is_some(), "copy choice should be pending");

    let mut state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) },
        &r,
    );
    assert!(!state.get_object(twin).unwrap().entering_copy_source,
        "declining must disarm the guard");
    // The game loop runs SBAs after the choice resolves; the disarmed 0/0 dies.
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "a declined Evil Twin is a 0/0 and dies to SBA");
}

/// With no other creature to copy, the guard is disarmed and the 0/0 dies.
#[test]
fn no_target_lets_the_0_0_die() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let twin = enter_twin(&mut state, &r);

    let behavior = r.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &r);
    assert!(!state.get_object(twin).unwrap().entering_copy_source,
        "no legal target must disarm the guard");
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "an Evil Twin with nothing to copy dies to SBA");
}

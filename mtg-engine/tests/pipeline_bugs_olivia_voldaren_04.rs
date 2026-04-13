mod common;

use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::types::*;
use common::{game_at_step, named_creature, ready_creature, P0, P1};

/// Oracle text: "{3}{B}{B}: Gain control of target Vampire for as long as you
/// control Olivia Voldaren."
///
/// "For as long as you control Olivia" means the control effect ends whenever
/// the player who activated the ability no longer controls Olivia — whether
/// Olivia leaves the battlefield OR has her controller changed while staying
/// on the battlefield (e.g. via Act of Treason, Zealous Conscripts).
///
/// The implementation (olivia_voldaren.rs:168-202) only reverts stolen creatures
/// in `on_leave_battlefield`. If an opponent steals Olivia without removing her,
/// the stolen Vampires remain under the original activator's control, violating
/// the "for as long as you control" condition.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn stolen_vampires_revert_when_olivia_changes_controller() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Olivia on the battlefield under P0's control.
    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);

    // Place a Vampire creature on the battlefield under P1's control.
    let vampire = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(vampire).unwrap().subtypes.push("Vampire".to_string());

    // Simulate P0 activating Olivia's second ability to steal the Vampire.
    // This mirrors olivia_voldaren.rs:133-162: set controller and record in card_state.
    {
        let original_controller = state.get_object(vampire).unwrap().controller;
        state.get_object_mut(vampire).unwrap().controller = P0;

        let olivia_obj = state.get_object_mut(olivia).unwrap();
        olivia_obj.card_state.insert("stolen_0".into(), vampire);
        olivia_obj.card_state.insert("orig_0".into(), ObjectId(original_controller.0 as u64));
    }

    // Sanity: P0 now controls the Vampire.
    assert_eq!(
        state.get_object(vampire).unwrap().controller, P0,
        "Sanity: P0 should control the stolen Vampire"
    );

    // Simulate an opponent stealing Olivia (e.g. Act of Treason / Zealous Conscripts).
    // Olivia stays on the battlefield but P1 now controls her.
    state.get_object_mut(olivia).unwrap().controller = P1;

    // Run state-based actions — this is where the engine should detect that P0
    // no longer controls Olivia and revert the stolen Vampire.
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // Oracle: "for as long as you control Olivia Voldaren" — P0 no longer
    // controls Olivia, so the control effect should have ended. The Vampire
    // should revert to P1's control.
    assert_eq!(
        state.get_object(vampire).unwrap().controller, P1,
        "Oracle: stolen Vampire should revert to P1 when P0 loses control of Olivia \
         (\"for as long as you control Olivia Voldaren\") — but only on_leave_battlefield \
         handles this, not controller changes"
    );
}

mod common;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use common::{game_at_step, named_creature, P0};

/// Rule 712.10: A double-faced card not on the battlefield has the
/// characteristics of its front face. When Lord of Lineage (transformed
/// Bloodline Keeper) dies, the card in the graveyard should have the name
/// "Bloodline Keeper", not "Lord of Lineage".
///
/// The engine clears `is_transformed` on zone change (state.rs:502) but
/// does NOT clear `obj.name`, so the back-face name persists in the
/// graveyard.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn dfc_name_reverts_to_front_face_on_leaving_battlefield() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Bloodline Keeper on the battlefield.
    let keeper = named_creature(&mut state, &registry, "Bloodline Keeper", P0);

    // Simulate transformation to Lord of Lineage (same as bloodline_keeper.rs:155-158).
    {
        let obj = state.get_object_mut(keeper).unwrap();
        obj.is_transformed = true;
        obj.name = "Lord of Lineage".into();
    }

    // Sanity: on the battlefield, the name is Lord of Lineage.
    assert_eq!(
        state.get_object(keeper).unwrap().name, "Lord of Lineage",
        "Sanity check: transformed creature should be named Lord of Lineage on battlefield"
    );

    // Move to graveyard (e.g., creature dies).
    state.move_object(keeper, Zone::Graveyard, &registry);

    // Verify is_transformed was cleared (the engine does this correctly).
    assert!(!state.get_object(keeper).unwrap().is_transformed,
        "is_transformed should be false after leaving battlefield");

    // Rule 712.10: A DFC not on the battlefield has front-face characteristics.
    // The name should revert to "Bloodline Keeper".
    assert_eq!(
        state.get_object(keeper).unwrap().name, "Bloodline Keeper",
        "Rule 712.10: DFC in graveyard should have front-face name 'Bloodline Keeper', \
         but engine retains back-face name 'Lord of Lineage' because move_object does not clear obj.name"
    );
}

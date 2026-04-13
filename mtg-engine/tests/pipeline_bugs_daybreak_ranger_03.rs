mod common;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

use common::{game_at_step, named_creature, P0, P1};

/// Rule 712.10: A double-faced card not on the battlefield or on the stack
/// has only its front-face characteristics. When Nightfall Predator (the
/// transformed back face of Daybreak Ranger) leaves the battlefield, the
/// zone-change cleanup in move_object clears is_transformed but does NOT
/// reset obj.name. The card should arrive in the graveyard with name
/// "Daybreak Ranger" (front face), but it keeps "Nightfall Predator".
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn dfc_name_resets_to_front_face_when_leaving_battlefield() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Daybreak Ranger on the battlefield for P0.
    let ranger_id = named_creature(&mut state, &registry, "Daybreak Ranger", P0);

    // Simulate the transform: upkeep with no spells cast last turn.
    // Set the condition so on_upkeep triggers the transform.
    state.is_first_turn = false;
    state.num_spells_cast_last_turn.clear();

    // Call on_upkeep to transform Daybreak Ranger -> Nightfall Predator.
    let behavior = registry.get(state.get_object(ranger_id).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, ranger_id, &registry);

    // Verify the transform happened: name should now be "Nightfall Predator"
    // and is_transformed should be true.
    let obj = state.get_object(ranger_id).unwrap();
    assert_eq!(obj.name, "Nightfall Predator", "Transform should set back-face name");
    assert!(obj.is_transformed, "Transform should set is_transformed flag");

    // Now move the transformed creature to the graveyard (e.g., it dies).
    state.move_object(ranger_id, Zone::Graveyard, &registry);

    // Per rule 712.10, the card in the graveyard should have its front-face
    // name "Daybreak Ranger", not the back-face name "Nightfall Predator".
    let obj_in_gy = state.get_object(ranger_id).unwrap();
    assert_eq!(obj_in_gy.zone, Zone::Graveyard, "Card should be in graveyard");
    assert!(!obj_in_gy.is_transformed, "is_transformed should be cleared in graveyard");
    assert_eq!(
        obj_in_gy.name, "Daybreak Ranger",
        "Rule 712.10: DFC in graveyard should have front-face name 'Daybreak Ranger', \
         but got '{}' because move_object clears is_transformed without resetting name",
        obj_in_gy.name
    );
}

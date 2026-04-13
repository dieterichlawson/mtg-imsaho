mod common;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use common::{game_at_step, ready_creature, P0, P1};

/// Rule 400.7: An object that changes zones becomes a new object with no
/// memory of its previous existence. Olivia Voldaren's first ability adds
/// "Vampire" to obj.subtypes, but when the creature dies and is later
/// returned to the battlefield, that subtype addition should not persist.
///
/// The engine's move_object (state.rs:494-505) clears tapped, counters,
/// damage, etc. on leaving the battlefield but does NOT clear obj.subtypes.
/// This causes the Vampire subtype to incorrectly survive zone changes.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn vampire_subtype_does_not_persist_across_zone_change() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place a generic 2/3 creature on the battlefield for P1.
    let creature = ready_creature(&mut state, P1, 2, 3);

    // Simulate Olivia's first ability: add "Vampire" subtype to the creature.
    // This mirrors olivia_voldaren.rs:116-118.
    {
        let obj = state.get_object_mut(creature).unwrap();
        assert!(!obj.subtypes.contains(&"Vampire".to_string()),
            "Sanity: creature should not start as a Vampire");
        obj.subtypes.push("Vampire".to_string());
    }

    // Sanity: creature is now a Vampire on the battlefield.
    assert!(
        state.get_object(creature).unwrap().subtypes.contains(&"Vampire".to_string()),
        "Sanity: creature should be a Vampire after Olivia's ability"
    );

    // Creature dies — move to graveyard.
    state.move_object(creature, Zone::Graveyard, &registry);

    // Creature is reanimated — move back to battlefield.
    state.move_object(creature, Zone::Battlefield, &registry);

    // Rule 400.7: The creature is a new object with no memory of its previous
    // existence. The Vampire subtype added by Olivia's ability should be gone.
    let has_vampire = state.get_object(creature).unwrap()
        .subtypes.contains(&"Vampire".to_string());
    assert_eq!(
        has_vampire, false,
        "Rule 400.7: creature returned to battlefield should not retain Vampire subtype \
         added by Olivia's ability — move_object does not clear obj.subtypes on zone change"
    );
}

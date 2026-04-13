//! Bug: Grimoire of the Dead's Zombie/Black modifications persist after zone change.
//!
//! MTG Rule 400.7: When an object changes zones, it becomes a new object with no
//! memory of its previous existence. The Zombie subtype and Black color added by
//! Grimoire's reanimation ability should not persist when the creature later dies
//! and moves back to the graveyard. The engine's move_object (state.rs:494-505)
//! clears tapped, summoning_sick, counters, etc. but does NOT clear subtypes or
//! colors, so these mutations incorrectly survive zone transitions.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::actions::Action;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

#[test]
#[ignore] // Pipeline bug — awaiting fix
fn grimoire_zombie_black_should_not_persist_after_leaving_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up Grimoire of the Dead on the battlefield with 3 study counters.
    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    state.add_counters(grimoire, CounterType::Study, 3);

    // Put a vanilla creature in the graveyard. It starts with no Zombie subtype
    // and no Black color.
    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().name = "Test Creature".into();
    assert!(!state.get_object(creature).unwrap().subtypes.contains(&"Zombie".into()),
        "precondition: creature should not start as Zombie");
    assert!(!state.get_object(creature).unwrap().colors.contains(&Color::Black),
        "precondition: creature should not start as Black");
    state.move_object(creature, Zone::Graveyard, &reg);

    // Activate Grimoire's second ability to reanimate all creatures from graveyards.
    // They become black Zombies in addition to their other colors and types.
    state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: grimoire,
            ability_index: 1,
            targets: vec![],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
        },
        &reg,
    );

    // Sanity check: creature is on the battlefield with Zombie subtype and Black color.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
    assert!(state.get_object(creature).unwrap().subtypes.contains(&"Zombie".into()),
        "sanity: creature should be a Zombie after Grimoire reanimation");
    assert!(state.get_object(creature).unwrap().colors.contains(&Color::Black),
        "sanity: creature should be Black after Grimoire reanimation");

    // The creature dies and moves back to the graveyard.
    state.move_object(creature, Zone::Graveyard, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);

    // Per MTG rule 400.7, the creature is now a new object with no memory of
    // Grimoire's modifications. It should NOT retain the Zombie subtype or Black
    // color that Grimoire added. These were runtime mutations, not inherent
    // characteristics of the card.
    let has_zombie = state.get_object(creature).unwrap().subtypes.contains(&"Zombie".into());
    assert_eq!(
        has_zombie, false,
        "Rule 400.7: creature should lose Zombie subtype after leaving the battlefield \
         — Grimoire's modification should not persist across zone changes"
    );
}

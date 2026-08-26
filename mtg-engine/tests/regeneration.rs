//! Regeneration (CR 701.15).
//!
//! "Regenerate this permanent" creates a shield. The next time it would be
//! destroyed this turn, instead tap it, remove it from combat, and remove all
//! damage marked on it — the shield is used up and the permanent survives.
//!
//! These are rules tests: no card is involved, only shields on a bare
//! permanent. They used to live in `cards_morbid_and_ltb.rs`, where they were
//! surrounded by tests about specific cards. The cards that *grant*
//! regeneration (Skeletal Grimace, Manor Skeleton) are tested with the other
//! cards; what the shield then does is here.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

#[test]
fn regeneration_shield_prevents_lethal_damage_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;
    state.get_object_mut(creature).unwrap().damage_marked = 2; // lethal

    check_state_based_actions(&mut state, &reg);

    // Should have regenerated, not died.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature with regeneration shield should not die from lethal damage");
    assert!(state.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "Regenerated creature should have damage removed");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "One shield should be consumed");
}

#[test]
fn regeneration_does_not_prevent_zero_toughness_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Creature with 0 effective toughness (e.g., from -2/-2 aura on a 2/2).
    let creature = ready_creature(&mut state, P0, 2, 0);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Regeneration should not save from 0 toughness");
}

#[test]
fn multiple_regeneration_shields() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().regeneration_shields = 2;
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 1,
        "One shield consumed, one remaining");

    // Deal lethal damage again.
    state.get_object_mut(creature).unwrap().damage_marked = 2;
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Second shield should save from second lethal");
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0);
}

#[test]
fn regeneration_shields_expire_at_cleanup() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;

    advance_to_cleanup(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Unused regeneration shields should expire at cleanup");
}

#[test]
fn try_destroy_respects_regeneration() {
    use mtg_engine::destruction::DestroyResult;
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;

    let result = mtg_engine::destruction::try_destroy(&mut state, creature, &reg);

    assert_eq!(result, DestroyResult::Regenerated);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
    assert!(state.get_object(creature).unwrap().tapped);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0);
}

#[test]
fn try_destroy_without_shield_kills() {
    use mtg_engine::destruction::DestroyResult;
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0);

    let result = mtg_engine::destruction::try_destroy(&mut state, creature, &reg);

    assert_eq!(result, DestroyResult::Died);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

#[test]
fn sacrifice_bypasses_regeneration() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().regeneration_shields = 2;

    let sacrificed = mtg_engine::destruction::sacrifice(&mut state, creature, &reg);

    assert!(sacrificed, "Sacrifice should succeed even with regeneration shields");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(creature).unwrap().regeneration_shields, 0,
        "Shields should be cleared when leaving battlefield");
}

#[test]
fn regeneration_saves_from_deathtouch() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 5, 5);
    state.get_object_mut(creature).unwrap().regeneration_shields = 1;
    state.get_object_mut(creature).unwrap().damage_marked = 1;
    state.get_object_mut(creature).unwrap().dealt_deathtouch_damage = true;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Regeneration should save from deathtouch damage");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0);
}

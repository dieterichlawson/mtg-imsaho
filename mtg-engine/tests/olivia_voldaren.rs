//! Tests for Olivia Voldaren.
//!
//! Oracle: {2}{B}{R} 3/3 Legendary Vampire, Flying
//! {1}{R}: Deal 1 damage to another target creature, make it a Vampire, +1/+1 counter on Olivia.
//! {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::types::*;
/// Ability 0: Deal 1 damage, make target a Vampire, +1/+1 counter on Olivia.
#[test]
fn olivia_ability_0_deals_damage_and_makes_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target).unwrap().subtypes = vec!["Human".into()];

    // Activate ability 0.
    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, olivia, 0, &[Target::Object(target)], &reg);

    // Target should have 1 damage and be a Vampire now.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 1, "Should deal 1 damage");
    assert!(target_obj.subtypes.contains(&"Vampire".to_string()),
        "Target should become a Vampire");
    assert!(target_obj.subtypes.contains(&"Human".to_string()),
        "Target should retain Human subtype");

    // Olivia should have a +1/+1 counter.
    assert_eq!(counters_of(&state, olivia, CounterType::PlusOnePlusOne), 1,
        "Olivia should have 1 +1/+1 counter");
}

/// Ability 0 can't target Olivia herself ("another").
#[test]
fn olivia_ability_0_cannot_target_self() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, olivia, 0, &[Target::Object(olivia)], &reg);

    // Olivia should have no +1/+1 counter (ability should be a no-op for self-target).
    assert_eq!(counters_of(&state, olivia, CounterType::PlusOnePlusOne), 0,
        "Olivia should not be able to target herself with ability 0");
}

/// Ability 1: Gain control of target Vampire.
#[test]
fn olivia_ability_1_steals_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(target).unwrap().subtypes = vec!["Vampire".into()];

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, olivia, 1, &[Target::Object(target)], &reg);

    // Target should now be controlled by P0.
    assert_eq!(state.get_object(target).unwrap().controller, P0,
        "Olivia should steal the Vampire");
}

/// Ability 1 should not steal non-Vampire creatures.
#[test]
fn olivia_ability_1_rejects_non_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(target).unwrap().subtypes = vec!["Human".into()];

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, olivia, 1, &[Target::Object(target)], &reg);

    // Target should still be controlled by P1 (not a Vampire).
    assert_eq!(state.get_object(target).unwrap().controller, P1,
        "Olivia should not steal a non-Vampire");
}

/// When Olivia leaves the battlefield, stolen creatures return to their original controller.
#[test]
fn olivia_stolen_creatures_return_when_olivia_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target1 = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(target1).unwrap().subtypes = vec!["Vampire".into()];
    let target2 = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target2).unwrap().subtypes = vec!["Vampire".into()];

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    // Steal both creatures.
    behavior.on_activate_ability(&mut state, olivia, 1, &[Target::Object(target1)], &reg);
    behavior.on_activate_ability(&mut state, olivia, 1, &[Target::Object(target2)], &reg);

    assert_eq!(state.get_object(target1).unwrap().controller, P0);
    assert_eq!(state.get_object(target2).unwrap().controller, P0);

    // Olivia leaves the battlefield. The control effect's condition is
    // checked as a state-based action, not by a handler on the card.
    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    // Both stolen creatures should return to P1.
    assert_eq!(state.get_object(target1).unwrap().controller, P1,
        "Stolen creature should return to original controller when Olivia leaves");
    assert_eq!(state.get_object(target2).unwrap().controller, P1,
        "Stolen creature should return to original controller when Olivia leaves");
}

/// Ability 1 target filter should only allow Vampires.
#[test]
fn olivia_ability_1_target_filter_requires_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(vampire).unwrap().subtypes = vec!["Vampire".into()];
    let human = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, olivia, &reg);

    // Ability 1 should have a Vampire target filter.
    let ability_1 = &abilities[1];
    match &ability_1.target_requirement {
        Some(mtg_engine::cards::TargetRequirement::CreatureWithFilter(
            mtg_engine::cards::TargetFilter::HasSubtype(s)
        )) => {
            assert_eq!(s, "Vampire", "Ability 1 should filter for Vampires");
        }
        other => panic!("Expected HasSubtype(Vampire) filter, got {other:?}"),
    }
}

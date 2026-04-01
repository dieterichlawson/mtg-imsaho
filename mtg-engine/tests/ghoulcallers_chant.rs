//! Tests for Ghoulcaller's Chant.
//!
//! Oracle: {B} Sorcery
//! Choose one —
//! • Return target creature card from your graveyard to your hand.
//! • Return two target Zombie creature cards from your graveyard to your hand.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Mode 1: Return a single creature card from your graveyard.
#[test]
fn mode1_returns_one_creature_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(creature, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let new_state = cast_and_resolve(&state, &reg, chant, vec![Target::Object(creature)]);

    assert_eq!(new_state.get_object(creature).unwrap().zone, Zone::Hand,
        "Mode 1 should return creature to hand");
}

/// Mode 2: Return two Zombie creature cards from your graveyard.
#[test]
fn mode2_returns_two_zombies_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let zombie1 = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(zombie1, Zone::Graveyard);
    let zombie2 = spell_in_hand(&mut state, &reg, "Diregraf Ghoul", P0);
    state.move_object(zombie2, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let new_state = cast_and_resolve(&state, &reg, chant, vec![
        Target::Object(zombie1),
        Target::Object(zombie2),
    ]);

    assert_eq!(new_state.get_object(zombie1).unwrap().zone, Zone::Hand,
        "Mode 2 should return first Zombie to hand");
    assert_eq!(new_state.get_object(zombie2).unwrap().zone, Zone::Hand,
        "Mode 2 should return second Zombie to hand");
}

/// Legal actions should include single-creature targets (mode 1).
#[test]
fn legal_actions_include_mode1_single_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a non-Zombie creature in graveyard.
    let bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(bear, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    let actions = engine::legal_actions(&state, &reg);
    let cast_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if object_id == &chant)
    }).collect();

    // Should have at least one action targeting the bear (mode 1).
    let has_bear_target = cast_actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            targets.len() == 1 && targets[0] == Target::Object(bear)
        } else {
            false
        }
    });
    assert!(has_bear_target, "Legal actions should include mode 1 targeting the non-Zombie creature");
}

/// Legal actions should include two-Zombie targets (mode 2).
#[test]
fn legal_actions_include_mode2_two_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let zombie1 = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(zombie1, Zone::Graveyard);
    let zombie2 = spell_in_hand(&mut state, &reg, "Diregraf Ghoul", P0);
    state.move_object(zombie2, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    let actions = engine::legal_actions(&state, &reg);
    let cast_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if object_id == &chant)
    }).collect();

    // Should have a 2-target action with both Zombies.
    let has_two_zombie = cast_actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            targets.len() == 2
                && targets.iter().all(|t| matches!(t, Target::Object(id) if *id == zombie1 || *id == zombie2))
        } else {
            false
        }
    });
    assert!(has_two_zombie, "Legal actions should include mode 2 targeting two Zombies");
}

/// Mode 2 should NOT be available for non-Zombie creatures.
#[test]
fn legal_actions_no_mode2_for_non_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two non-Zombie creatures in graveyard.
    let bear1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(bear1, Zone::Graveyard);
    let bear2 = spell_in_hand(&mut state, &reg, "Savannah Lions", P0);
    state.move_object(bear2, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    let actions = engine::legal_actions(&state, &reg);
    let cast_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if object_id == &chant)
    }).collect();

    // Should NOT have any 2-target actions (no Zombies).
    let has_two_target = cast_actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            targets.len() == 2
        } else {
            false
        }
    });
    assert!(!has_two_target,
        "Legal actions should not include mode 2 when no Zombies are in graveyard");

    // But should still have mode 1 actions.
    let has_one_target = cast_actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            targets.len() == 1
        } else {
            false
        }
    });
    assert!(has_one_target,
        "Legal actions should still include mode 1 for non-Zombie creatures");
}

/// Cannot target opponent's graveyard creatures.
#[test]
fn cannot_target_opponents_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put creature in opponent's graveyard only.
    let enemy_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(enemy_creature, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    let actions = engine::legal_actions(&state, &reg);
    let cast_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if object_id == &chant)
    }).collect();

    // Should have no cast actions (no valid targets in P0's graveyard).
    assert!(cast_actions.is_empty(),
        "Should not be able to target opponent's graveyard creatures");
}

/// Mixed graveyard: mode 1 for any creature, mode 2 only for Zombies.
#[test]
fn mixed_graveyard_correct_modes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // One Zombie and one non-Zombie.
    let zombie = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(zombie, Zone::Graveyard);
    let bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(bear, Zone::Graveyard);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);

    let actions = engine::legal_actions(&state, &reg);
    let cast_actions: Vec<_> = actions.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if object_id == &chant)
    }).collect();

    // Mode 1 actions: should have both bear and zombie as single targets.
    let single_targets: Vec<_> = cast_actions.iter().filter_map(|a| {
        if let Action::CastSpell { targets, .. } = a {
            if targets.len() == 1 { Some(targets[0].clone()) } else { None }
        } else {
            None
        }
    }).collect();
    assert!(single_targets.contains(&Target::Object(bear)), "Mode 1 should include bear");
    assert!(single_targets.contains(&Target::Object(zombie)), "Mode 1 should include zombie");

    // Mode 2: should NOT have 2-target actions (only 1 Zombie available).
    let has_two_target = cast_actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            targets.len() == 2
        } else {
            false
        }
    });
    assert!(!has_two_target,
        "Mode 2 should not be available with only 1 Zombie in graveyard");
}

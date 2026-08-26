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

/// Mode 1: Return a single creature card from your graveyard.
#[test]
fn mode1_returns_one_creature_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);

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

    let zombie1 = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let zombie2 = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);

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
    let bear = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);

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

    let zombie1 = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let zombie2 = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);

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
    let _bear1 = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let _bear2 = named_card_in_graveyard(&mut state, &reg, "Savannah Lions", P0);

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
    let _enemy_creature = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);

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
    let zombie = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let bear = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);

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

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Ghoulcaller's Chant is modal with two modes. The engine's
/// `build_cast_target_spec` may not handle modal spells containing
/// `TwoTargets` correctly, causing incorrect action generation.
#[test]
fn bug_ghoulcallers_chant_modal_targeting() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a creature and two Zombies in P0's graveyard
    let _bear = {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears".into();
        id
    };
    let _zombie1 = {
        let card_id = registry.get_id_by_name("Walking Corpse").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Walking Corpse".into();
        id
    };
    let _zombie2 = {
        let card_id = registry.get_id_by_name("Diregraf Ghoul").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Diregraf Ghoul".into();
        id
    };

    // Cast Ghoulcaller's Chant
    let chant = castable_spell(&mut state, &registry, "Ghoulcaller's Chant", P0);

    // Get legal actions — should have actions for BOTH modes
    let legal = engine::legal_actions(&state, &registry);
    let chant_actions: Vec<_> = legal.actions.iter().filter(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == chant)
    }).collect();

    // Should have at least mode 1 (return any creature: bear, zombie1, zombie2 = 3 actions)
    // plus mode 2 (return two Zombies: zombie1+zombie2 = 1 action)
    // BUG: Modal targeting spec may not generate actions for both modes
    assert!(chant_actions.len() >= 4,
        "Should have actions for both modes (3 creature + 1 two-zombie). Got: {}",
        chant_actions.len());
}

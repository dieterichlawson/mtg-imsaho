//! Tests for Infernal Plunge.
//!
//! Oracle: {R} Sorcery
//! As an additional cost to cast this spell, sacrifice a creature.
//! Add {R}{R}{R}.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Cannot cast Infernal Plunge without a creature to sacrifice.
#[test]
fn cannot_cast_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if {
            state.get_object(*object_id)
                .map(|o| o.name == "Infernal Plunge")
                .unwrap_or(false)
        })
    });

    assert!(!can_cast,
        "Should not be able to cast Infernal Plunge without a creature to sacrifice");
}

/// Can cast Infernal Plunge when controlling a creature.
#[test]
fn can_cast_with_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let _creature = ready_creature(&mut state, P0, 1, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if {
            state.get_object(*object_id)
                .map(|o| o.name == "Infernal Plunge")
                .unwrap_or(false)
        })
    });

    assert!(can_cast,
        "Should be able to cast Infernal Plunge when controlling a creature");
}

/// Sacrifice happens at cast time (creature is gone before resolution).
#[test]
fn sacrifice_at_cast_time() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast with explicit sacrifice target.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge,
            targets: vec![],
            sacrifice: Some(creature),
        },
        &reg,
    );

    // Creature should already be in graveyard (sacrificed at cast time).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature should be sacrificed at cast time, not resolution");

    // Spell should be on the stack.
    assert_eq!(state.get_object(plunge).unwrap().zone, Zone::Stack,
        "Infernal Plunge should be on the stack");
}

/// On resolution, adds {R}{R}{R} to mana pool.
#[test]
fn adds_three_red_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast with explicit sacrifice target.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge,
            targets: vec![],
            sacrifice: Some(creature),
        },
        &reg,
    );

    // Record mana before resolution (should be 0 since we spent {R} to cast).
    let red_before = state.get_player(P0).mana_pool.get(ManaType::Red);

    // Resolve the spell.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let red_after = state.get_player(P0).mana_pool.get(ManaType::Red);
    assert_eq!(red_after - red_before, 3,
        "Infernal Plunge should add RRR on resolution");
}

/// Legal actions show one CastSpell per eligible creature to sacrifice.
#[test]
fn one_action_per_sacrifice_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 3, 3);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let plunge_actions: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, sacrifice, .. } = a {
            state.get_object(*object_id)
                .map(|o| o.name == "Infernal Plunge")
                .unwrap_or(false)
            && sacrifice.is_some()
        } else {
            false
        }
    }).collect();

    assert_eq!(plunge_actions.len(), 2,
        "Should have one CastSpell action per eligible creature (got {})", plunge_actions.len());

    // Both creatures should be represented as sacrifice options.
    let sac_ids: Vec<_> = plunge_actions.iter().filter_map(|a| {
        if let Action::CastSpell { sacrifice, .. } = a {
            *sacrifice
        } else {
            None
        }
    }).collect();
    assert!(sac_ids.contains(&creature_a), "Should include creature A as sacrifice option");
    assert!(sac_ids.contains(&creature_b), "Should include creature B as sacrifice option");
}

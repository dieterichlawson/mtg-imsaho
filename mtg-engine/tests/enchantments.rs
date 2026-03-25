//! Tests for enchantments, auras, and continuous effects.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::types::*;

/// Holy Strength attaches to a creature and gives +1/+2.
#[test]
fn holy_strength_buffs_creature() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs_id = registry.get_id_by_name("Holy Strength").unwrap();
    let hs = state.create_object(hs_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);

    // Cast Holy Strength targeting creature.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: hs, targets: vec![Target::Object(creature)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Aura should be on battlefield, attached to creature.
    assert_eq!(state.get_object(hs).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(hs).unwrap().attached_to, Some(creature));

    // Creature should have effective +1/+2.
    assert_eq!(state.effective_power(creature, &registry), Some(3));
    assert_eq!(state.effective_toughness(creature, &registry), Some(4));
}

/// Aura falls off when enchanted creature dies.
#[test]
fn aura_falls_off_when_creature_dies() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs_id = registry.get_id_by_name("Holy Strength").unwrap();
    let hs = state.create_object(hs_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);

    // Attach Holy Strength.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: hs, targets: vec![Target::Object(creature)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Kill the creature.
    state.move_object(creature, Zone::Graveyard);

    // SBA should put the unattached aura in graveyard.
    check_state_based_actions_with_registry(&mut state, Some(&registry));
    assert_eq!(state.get_object(hs).unwrap().zone, Zone::Graveyard);
}

/// Pacifism prevents a creature from attacking.
#[test]
fn pacifism_prevents_attacking() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    let pac_id = registry.get_id_by_name("Pacifism").unwrap();
    let pac = state.create_object(pac_id, P1, Zone::Hand, None, None);
    state.get_player_mut(P1).mana_pool.add(ManaType::White, 2);
    state.priority_player = Some(P1); // P1 casts it

    // Cast Pacifism on P0's creature.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: pac, targets: vec![Target::Object(creature)] },
        &registry,
    );
    state.priority_player = Some(P0); // back to P0
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Creature should not be able to attack.
    assert!(!state.can_attack(creature, &registry));

    // And should not be able to block.
    assert!(!state.can_block(creature, &registry));
}

/// Glorious Anthem gives +1/+1 to all your creatures.
#[test]
fn glorious_anthem_buffs_all_creatures() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 2);
    let c2 = ready_creature(&mut state, P0, 1, 1);
    let opp_creature = ready_creature(&mut state, P1, 3, 3);

    // Put Glorious Anthem on battlefield.
    let anthem_id = registry.get_id_by_name("Glorious Anthem").unwrap();
    let anthem = state.create_object(anthem_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: anthem, targets: vec![] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);
    assert_eq!(state.get_object(anthem).unwrap().zone, Zone::Battlefield);

    // Your creatures should get +1/+1.
    assert_eq!(state.effective_power(c1, &registry), Some(3));
    assert_eq!(state.effective_toughness(c1, &registry), Some(3));
    assert_eq!(state.effective_power(c2, &registry), Some(2));
    assert_eq!(state.effective_toughness(c2, &registry), Some(2));

    // Opponent's creature should NOT get the bonus.
    assert_eq!(state.effective_power(opp_creature, &registry), Some(3));
    assert_eq!(state.effective_toughness(opp_creature, &registry), Some(3));
}

/// Giant Growth gives +3/+3 until end of turn, then wears off.
#[test]
fn giant_growth_until_end_of_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let gg_id = registry.get_id_by_name("Giant Growth").unwrap();
    let gg = state.create_object(gg_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    // Cast Giant Growth on creature.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: gg, targets: vec![Target::Object(creature)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Should have +3/+3 (effective 5/5).
    assert_eq!(state.effective_power(creature, &registry), Some(5));
    assert_eq!(state.effective_toughness(creature, &registry), Some(5));

    // Advance to cleanup — effect should wear off.
    loop {
        engine::advance_step(&mut state, &registry);
        if state.step == Step::Cleanup {
            break;
        }
    }

    // Effect should be gone.
    assert_eq!(state.until_end_of_turn_effects.len(), 0);
    assert_eq!(state.effective_power(creature, &registry), Some(2));
    assert_eq!(state.effective_toughness(creature, &registry), Some(2));
}

/// A creature with Holy Strength (+1/+2) survives damage that would
/// kill its base toughness, when checked with the registry.
#[test]
fn aura_toughness_bonus_prevents_death() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // 2/2 creature with Holy Strength = effective 3/4.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs_id = registry.get_id_by_name("Holy Strength").unwrap();
    let hs = state.create_object(hs_id, P0, Zone::Hand, None, None);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: hs, targets: vec![Target::Object(creature)] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Deal 3 damage — enough to kill a 2/2 but not a 3/4.
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    check_state_based_actions_with_registry(&mut state, Some(&registry));
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "3/4 creature with 3 damage should survive");

    // Deal 4th damage — now it dies.
    state.get_object_mut(creature).unwrap().damage_marked = 4;
    check_state_based_actions_with_registry(&mut state, Some(&registry));
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "3/4 creature with 4 damage should die");
}

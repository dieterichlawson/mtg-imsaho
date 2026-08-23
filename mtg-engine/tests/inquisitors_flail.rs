//! Tests for Inquisitor's Flail.
//!
//! Oracle: {2} Artifact — Equipment
//! If equipped creature would deal combat damage, it deals double that damage instead.
//! If another source would deal combat damage to equipped creature, it deals double
//! that damage to equipped creature instead.
//! Equip {2}

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Equipped creature deals double combat damage to a player.
#[test]
fn doubles_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 4, 4);
    state.get_object_mut(flail).unwrap().attached_to = Some(creature);

    // Creature attacks P1 unblocked.
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(creature, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });

    let life_before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    let life_after = state.get_player(P1).life;

    assert_eq!(life_before - life_after, 8,
        "4 power with Flail should deal 8 damage to player");
}

/// Equipped creature deals double combat damage to a blocking creature.
#[test]
fn doubles_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 5, 5);
    state.get_object_mut(flail).unwrap().attached_to = Some(attacker);

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(attacker, P1)].into_iter().collect(),
        blocker_assignments: [(attacker, vec![blocker])].into_iter().collect(),
        ..Default::default()
    });

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let blocker_damage = state.get_object(blocker).unwrap().damage_marked;
    assert_eq!(blocker_damage, 4,
        "2 power with Flail should deal 4 damage to blocker");
}

/// Equipped creature takes double combat damage from blockers.
#[test]
fn doubles_damage_taken_from_blocker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let attacker = ready_creature(&mut state, P0, 3, 6);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(flail).unwrap().attached_to = Some(attacker);

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(attacker, P1)].into_iter().collect(),
        blocker_assignments: [(attacker, vec![blocker])].into_iter().collect(),
        ..Default::default()
    });

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    let attacker_damage = state.get_object(attacker).unwrap().damage_marked;
    assert_eq!(attacker_damage, 4,
        "Equipped creature should take double damage from 2-power blocker (4 instead of 2)");
}

/// Without Flail equipped, damage is normal.
#[test]
fn no_doubling_without_flail() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let _flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    // Flail NOT attached.
    let creature = ready_creature(&mut state, P0, 3, 3);

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(creature, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });

    let life_before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    let life_after = state.get_player(P1).life;

    assert_eq!(life_before - life_after, 3,
        "Without Flail equipped, damage should be normal");
}

/// Per ruling: two Inquisitor's Flails should quadruple damage (2^2 = 4x).
#[test]
fn two_flails_quadruple_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let flail1 = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let flail2 = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    state.get_object_mut(flail1).unwrap().attached_to = Some(creature);
    state.get_object_mut(flail2).unwrap().attached_to = Some(creature);

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(creature, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });

    let life_before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    let life_after = state.get_player(P1).life;

    assert_eq!(life_before - life_after, 12,
        "Two Flails should quadruple damage: 3 * 4 = 12");
}

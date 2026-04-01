//! Tests for Kruin Outlaw / Terror of Kruin Pass.
//!
//! Terror of Kruin Pass (back face):
//! - Double strike
//! - Each Werewolf you control can't be blocked except by two or more creatures.
//! - Transform back if a player cast 2+ spells last turn.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Terror of Kruin Pass grants "can't be blocked except by two or more creatures"
/// to all Werewolves you control, including itself.
#[test]
fn terror_of_kruin_pass_self_requires_two_blockers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // Put Terror of Kruin Pass on the battlefield (transformed).
    let terror = named_creature(&mut state, &reg, "Kruin Outlaw", P0);
    if let Some(obj) = state.get_object_mut(terror) {
        obj.is_transformed = true;
        obj.name = "Terror of Kruin Pass".into();
    }

    // Set up combat with Terror attacking.
    let mut combat_state = mtg_engine::state::CombatState {
        attackers: std::collections::HashMap::new(),
        blocker_assignments: std::collections::HashMap::new(),
    };
    combat_state.attackers.insert(terror, P1);
    combat_state.blocker_assignments.insert(terror, vec![]);
    state.combat = Some(combat_state);

    // Defender has one creature.
    let blocker1 = ready_creature(&mut state, P1, 2, 2);

    // Try to block with only 1 creature — should be rejected.
    combat::declare_blockers_with_registry(&mut state, &[(blocker1, terror)], &reg);
    let blockers = state.combat.as_ref().unwrap()
        .blocker_assignments.get(&terror).unwrap();
    assert_eq!(blockers.len(), 0,
        "Terror of Kruin Pass should reject a single blocker");
}

/// Terror of Kruin Pass allows blocking by 2+ creatures.
#[test]
fn terror_of_kruin_pass_allows_two_blockers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let terror = named_creature(&mut state, &reg, "Kruin Outlaw", P0);
    if let Some(obj) = state.get_object_mut(terror) {
        obj.is_transformed = true;
        obj.name = "Terror of Kruin Pass".into();
    }

    let mut combat_state = mtg_engine::state::CombatState {
        attackers: std::collections::HashMap::new(),
        blocker_assignments: std::collections::HashMap::new(),
    };
    combat_state.attackers.insert(terror, P1);
    combat_state.blocker_assignments.insert(terror, vec![]);
    state.combat = Some(combat_state);

    let blocker1 = ready_creature(&mut state, P1, 2, 2);
    let blocker2 = ready_creature(&mut state, P1, 2, 2);

    combat::declare_blockers_with_registry(
        &mut state,
        &[(blocker1, terror), (blocker2, terror)],
        &reg,
    );
    let blockers = state.combat.as_ref().unwrap()
        .blocker_assignments.get(&terror).unwrap();
    assert_eq!(blockers.len(), 2,
        "Terror of Kruin Pass should allow 2 blockers");
}

/// Terror of Kruin Pass grants the blocking restriction to OTHER Werewolves you control.
#[test]
fn terror_of_kruin_pass_grants_restriction_to_other_werewolves() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // Terror of Kruin Pass on the battlefield (transformed).
    let terror = named_creature(&mut state, &reg, "Kruin Outlaw", P0);
    if let Some(obj) = state.get_object_mut(terror) {
        obj.is_transformed = true;
        obj.name = "Terror of Kruin Pass".into();
    }

    // Another Werewolf creature controlled by P0.
    let other_wolf = ready_creature(&mut state, P0, 3, 3);
    if let Some(obj) = state.get_object_mut(other_wolf) {
        obj.subtypes = vec!["Werewolf".into()];
    }

    // Set up combat with the other Werewolf attacking.
    let mut combat_state = mtg_engine::state::CombatState {
        attackers: std::collections::HashMap::new(),
        blocker_assignments: std::collections::HashMap::new(),
    };
    combat_state.attackers.insert(other_wolf, P1);
    combat_state.blocker_assignments.insert(other_wolf, vec![]);
    state.combat = Some(combat_state);

    let blocker1 = ready_creature(&mut state, P1, 2, 2);

    // Try to block with only 1 creature — should be rejected.
    combat::declare_blockers_with_registry(&mut state, &[(blocker1, other_wolf)], &reg);
    let blockers = state.combat.as_ref().unwrap()
        .blocker_assignments.get(&other_wolf).unwrap();
    assert_eq!(blockers.len(), 0,
        "Other Werewolves should also require 2+ blockers when Terror of Kruin Pass is on battlefield");
}

/// Non-Werewolf creatures should NOT get the blocking restriction from Terror of Kruin Pass.
#[test]
fn terror_of_kruin_pass_does_not_affect_non_werewolves() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let terror = named_creature(&mut state, &reg, "Kruin Outlaw", P0);
    if let Some(obj) = state.get_object_mut(terror) {
        obj.is_transformed = true;
        obj.name = "Terror of Kruin Pass".into();
    }

    // A non-Werewolf creature.
    let human = ready_creature(&mut state, P0, 3, 3);
    if let Some(obj) = state.get_object_mut(human) {
        obj.subtypes = vec!["Human".into()];
    }

    let mut combat_state = mtg_engine::state::CombatState {
        attackers: std::collections::HashMap::new(),
        blocker_assignments: std::collections::HashMap::new(),
    };
    combat_state.attackers.insert(human, P1);
    combat_state.blocker_assignments.insert(human, vec![]);
    state.combat = Some(combat_state);

    let blocker1 = ready_creature(&mut state, P1, 2, 2);

    // Blocking with 1 creature should be fine for non-Werewolves.
    combat::declare_blockers_with_registry(&mut state, &[(blocker1, human)], &reg);
    let blockers = state.combat.as_ref().unwrap()
        .blocker_assignments.get(&human).unwrap();
    assert_eq!(blockers.len(), 1,
        "Non-Werewolf should be blockable by 1 creature");
}

/// Opponent's Werewolves should NOT get the restriction.
#[test]
fn terror_of_kruin_pass_does_not_affect_opponent_werewolves() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);
    state.active_player = P1;

    let terror = named_creature(&mut state, &reg, "Kruin Outlaw", P0);
    if let Some(obj) = state.get_object_mut(terror) {
        obj.is_transformed = true;
        obj.name = "Terror of Kruin Pass".into();
    }

    // Opponent's Werewolf.
    let opp_wolf = ready_creature(&mut state, P1, 3, 3);
    if let Some(obj) = state.get_object_mut(opp_wolf) {
        obj.subtypes = vec!["Werewolf".into()];
    }

    let mut combat_state = mtg_engine::state::CombatState {
        attackers: std::collections::HashMap::new(),
        blocker_assignments: std::collections::HashMap::new(),
    };
    combat_state.attackers.insert(opp_wolf, P0);
    combat_state.blocker_assignments.insert(opp_wolf, vec![]);
    state.combat = Some(combat_state);

    let blocker1 = ready_creature(&mut state, P0, 2, 2);

    // Opponent's Werewolf should be blockable by 1 creature.
    combat::declare_blockers_with_registry(&mut state, &[(blocker1, opp_wolf)], &reg);
    let blockers = state.combat.as_ref().unwrap()
        .blocker_assignments.get(&opp_wolf).unwrap();
    assert_eq!(blockers.len(), 1,
        "Opponent's Werewolves should not be affected by our Terror of Kruin Pass");
}

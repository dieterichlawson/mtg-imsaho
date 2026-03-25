//! Tests for combat rules: attacking, blocking, damage assignment.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::AwaitingAction;
use mtg_engine::types::*;

/// Rule 509.1h: A blocked creature remains blocked even if all blockers
/// are removed. Without trample, it deals no damage to the player.
/// Rule 510.1c: A blocked creature with no remaining blockers assigns
/// no combat damage.
#[test]
fn blocked_creature_with_removed_blocker_deals_no_damage() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 5, 5);
    let blocker = ready_creature(&mut state, P1, 1, 1);

    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    // Blocker is removed before damage (e.g., by a spell).
    state.move_object(blocker, Zone::Graveyard);

    combat::deal_combat_damage(&mut state);

    assert_eq!(state.get_player(P1).life, 20,
        "Blocked creature with removed blocker deals no damage (rules 509.1h, 510.1c)");
}

/// Rule 302.6: Summoning sickness does NOT prevent blocking.
#[test]
fn summoning_sick_creature_can_block() {
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let blocker = sick_creature(&mut state, P1, 3, 3);
    assert!(state.get_object(blocker).unwrap().summoning_sick);

    let blockers = combat::eligible_blockers(&state, P1);
    assert!(blockers.contains(&blocker),
        "Summoning sick creature should be able to block (rule 302.6)");
}

/// Rule 302.6: Summoning sickness DOES prevent attacking.
#[test]
fn summoning_sick_creature_cannot_attack() {
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);

    let attackers = combat::eligible_attackers(&state, P0);
    assert!(!attackers.contains(&creature),
        "Summoning sick creature should not be able to attack (rule 302.6)");
}

/// Rule 508.1f: Attacking causes tapping — it's not a cost.
#[test]
fn attacking_taps_the_creature() {
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    assert!(!state.get_object(attacker).unwrap().tapped);

    combat::declare_attackers(&mut state, &[(attacker, P1)]);

    assert!(state.get_object(attacker).unwrap().tapped,
        "Creature should be tapped after declaring as attacker (rule 508.1f)");
}

/// Tapped creatures can't block.
#[test]
fn tapped_creature_cannot_block() {
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(creature).unwrap().tapped = true;

    let blockers = combat::eligible_blockers(&state, P1);
    assert!(!blockers.contains(&creature),
        "Tapped creature should not be able to block");
}

/// Multiple blockers on one attacker — all blockers deal damage to attacker,
/// attacker deals damage to each blocker.
#[test]
fn multiple_blockers_all_deal_damage_to_attacker() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 5, 5);
    let blocker_a = ready_creature(&mut state, P1, 2, 2);
    let blocker_b = ready_creature(&mut state, P1, 3, 3);

    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    combat::declare_blockers(&mut state, &[(blocker_a, attacker), (blocker_b, attacker)]);
    combat::deal_combat_damage(&mut state);

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 5,
        "Attacker should take 2+3=5 damage from both blockers");
    assert_eq!(state.get_player(P1).life, 20,
        "Blocked attacker should not deal damage to player");
}

/// Creature with 0 power deals no combat damage.
#[test]
fn zero_power_creature_deals_no_damage() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 0, 3);

    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    combat::declare_blockers(&mut state, &[]);
    combat::deal_combat_damage(&mut state);

    assert_eq!(state.get_player(P1).life, 20,
        "0-power creature should deal no combat damage");
}

/// Declaring zero attackers is always valid — the player can submit
/// an empty attacker list from the combat prompt.
#[test]
fn declaring_zero_attackers_is_legal() {
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    ready_creature(&mut state, P0, 3, 3);

    let registry = CardRegistry::with_all_cards();
    let legal = engine::legal_actions(&state, &registry);

    // Should have a combat prompt with eligible attackers.
    assert!(legal.combat_prompt.is_some(),
        "Should have a combat prompt for declare attackers");
    // An empty DeclareAttackers action is always valid to submit.
    let action = Action::DeclareAttackers { attackers: vec![] };
    // Should not panic when submitted.
    let _new_state = engine::submit_action(&state, &action, &registry);
}

/// Full combat: two attackers, one blocked, one gets through.
#[test]
fn full_combat_with_partial_block() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker_a = ready_creature(&mut state, P0, 3, 3);
    let attacker_b = ready_creature(&mut state, P0, 2, 1);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    combat::declare_attackers(&mut state, &[(attacker_a, P1), (attacker_b, P1)]);
    assert!(state.get_object(attacker_a).unwrap().tapped);
    assert!(state.get_object(attacker_b).unwrap().tapped);

    combat::declare_blockers(&mut state, &[(blocker, attacker_a)]);
    combat::deal_combat_damage(&mut state);

    // Attacker A and blocker trade.
    assert_eq!(state.get_object(attacker_a).unwrap().damage_marked, 3);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3);

    // Attacker B unblocked — 2 damage to P1.
    assert_eq!(state.get_player(P1).life, 18);

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(attacker_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(attacker_b).unwrap().zone, Zone::Battlefield);
}

/// A creature that attacks and is blocked by a smaller creature survives
/// if the blocker's power is less than the attacker's toughness.
#[test]
fn attacker_survives_small_blocker() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 5, 5);
    let blocker = ready_creature(&mut state, P1, 1, 1);

    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    combat::deal_combat_damage(&mut state);

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 1);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 5);

    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Battlefield,
        "5/5 should survive being blocked by 1/1");
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard,
        "1/1 should die from 5 damage");
}

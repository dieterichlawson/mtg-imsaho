//! Regressions in the combat pipeline: declaring, blocking, and damage.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::engine;
use mtg_engine::state::AwaitingAction;
use mtg_engine::types::*;


/// A tapped creature can't be declared as a blocker (CR 509.1a). The
/// validating gate must drop it, leaving the attacker unblocked.
#[test]
fn an_illegal_block_by_a_tapped_creature_does_not_absorb_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().tapped = true;
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &reg);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "a tapped creature isn't blocking, so it takes no combat damage");
    assert_eq!(state.get_player(P1).life, p1_life - 2,
        "the block was illegal; the attacker is unblocked and hits the player");
}

/// A creature the attacking player controls can't be declared as a blocker —
/// only the defending player's creatures block (CR 509.1a).
#[test]
fn attacking_players_own_creature_cannot_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let fake_blocker = ready_creature(&mut state, P0, 2, 2); // controlled by the attacker's player
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(fake_blocker, attacker)], &reg);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, p1_life - 2,
        "a creature controlled by the attacker can't block; attacker is unblocked");
}

/// The DeclareAttackers handler validates eligibility: a summoning-sick
/// creature (no haste) submitted as an attacker is dropped.
#[test]
fn ineligible_attacker_is_filtered_by_the_handler() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let sick = sick_creature(&mut state, P0, 2, 2);
    let ready = ready_creature(&mut state, P0, 3, 3);
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);

    let state = engine::submit_action(
        &state,
        &Action::DeclareAttackers { attackers: vec![(sick, P1), (ready, P1)] },
        &reg,
    );

    let attacking: Vec<_> = state.combat.as_ref()
        .map(|c| c.attackers.keys().copied().collect())
        .unwrap_or_default();
    assert!(attacking.contains(&ready), "the eligible creature attacks");
    assert!(!attacking.contains(&sick),
        "a summoning-sick creature without haste can't be declared as an attacker");
}

/// A blocker that regenerates away first-strike lethal damage is removed
/// from combat and must not deal its damage in the regular step.
#[test]
fn regenerated_blocker_deals_no_regular_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().regeneration_shields = 1;
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // First strike killed the blocker; it regenerated (tapped, healed,
    // removed from combat).
    let b = state.get_object(blocker).unwrap();
    assert_eq!(b.zone, Zone::Battlefield, "blocker should have regenerated");
    assert!(b.tapped, "regeneration taps the creature");
    assert_eq!(b.regeneration_shields, 0);

    // CR 701.15c: the regenerated creature was removed from combat and must
    // NOT deal regular combat damage to the attacker.
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "attacker must take no damage from a blocker that left combat");
    // The attacker remains blocked (no trample): the player takes nothing.
    assert_eq!(state.get_player(P1).life, p1_life);
}

/// A double-striker whose blocker regenerated away stays BLOCKED
/// (CR 510.1c): its regular-step damage hits nothing — not the player.
#[test]
fn double_striker_stays_blocked_when_blocker_leaves_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::DoubleStrike);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().regeneration_shields = 1;
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // Blocker regenerated away the first-strike damage and left combat.
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "regeneration clears marked damage; regular-step damage must not land");
    // The attacker is still blocked and has no trample: regular-step damage
    // is assigned to nothing — the defending player takes none.
    assert_eq!(state.get_player(P1).life, p1_life,
        "blocked double-striker must not hit the player when its blocker leaves combat");
}

/// CR 510.5: with first strikers in combat there are TWO combat damage
/// steps, with SBAs and a priority round between them. The engine models
/// this by repeating Step::CombatDamage.
#[test]
fn first_strike_creates_second_combat_damage_step_with_window() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // 2/2 first striker attacks; 4/4 blocks (survives first strike).
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 4, 4);

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    // Enter the combat damage step: FIRST instance — first-strike damage only.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    assert!(state.combat_damage_step_pending,
        "a second combat damage step must be pending (CR 510.5)");
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2,
        "first striker deals its damage in the first step");
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "non-first-striker deals nothing in the first step");

    // Priority window between the steps: the defender removes the attacker
    // (as a Doom Blade would during this round of priority).
    state.move_object(attacker, Zone::Graveyard, &reg);

    // All players pass: the step repeats — SECOND instance, regular damage.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage,
        "Step::CombatDamage must repeat for the regular damage step");
    assert!(!state.combat_damage_step_pending);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2,
        "the removed attacker deals no regular damage; blocker keeps only first-strike damage");

    // And the step sequence continues normally afterwards.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::EndCombat);
}

/// Without first strikers, the combat damage step happens exactly once.
#[test]
fn no_first_strike_single_combat_damage_step() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    assert!(!state.combat_damage_step_pending,
        "no first strikers: no second damage step");
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 2);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2);

    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::EndCombat);
}

/// First-strike deaths produce their triggers BEFORE regular damage: the
/// window lets death triggers resolve between the two damage steps.
#[test]
fn first_strike_kill_prevents_regular_damage_back() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // 2/2 first striker vs 2/2 blocker: blocker dies to first strike and
    // never deals regular damage back.
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    mtg_engine::engine::advance_step(&mut state, &reg);
    // The game loop runs SBAs before granting priority (CR 117.5).
    while mtg_engine::sba::check_state_based_actions(&mut state, &reg) {}
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard,
        "blocker dies to first-strike damage before the regular step");

    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "dead blocker deals no regular-step damage");
}

/// Blazing Torch's granted ability must be offered to the equipped
/// creature's controller only when that player also controls the Torch —
/// its cost sacrifices the Torch, which only its controller may do.
#[test]
fn opponents_equipment_grants_no_activatable_ability() {
    let reg = registry();

    // Case 1: creature and torch share a controller — ability offered.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    let legal = engine::legal_actions(&state, &reg);
    let torch_ability_offered = legal.actions.iter().any(|a| matches!(
        a, Action::ActivateAbility { object_id, ability_index: 1, .. } if *object_id == creature));
    assert!(torch_ability_offered,
        "own torch on own creature: granted ability should be offered");

    // Case 2: the torch belongs to the opponent — ability must NOT be offered.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P1);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    let legal = engine::legal_actions(&state, &reg);
    let torch_ability_offered = legal.actions.iter().any(|a| matches!(
        a, Action::ActivateAbility { object_id, ability_index: 1, .. } if *object_id == creature));
    assert!(!torch_ability_offered,
        "opponent's torch: the sacrifice cost is unpayable, ability must not be offered");
}

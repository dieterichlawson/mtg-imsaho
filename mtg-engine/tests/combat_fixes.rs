//! Regression tests for combat point fixes:
//! - Creatures removed from combat between the first-strike and regular
//!   damage steps (e.g. by regenerating) must not deal or receive regular
//!   combat damage (CR 701.15c) — previously `deal_combat_damage` iterated
//!   a stale snapshot of the combat state taken before the SBA pass.
//! - Abilities granted by an attached permanent are only offered to a
//!   player who controls the attachment (CR 601.2g/701.13 — the granted
//!   abilities in this set all sacrifice the attachment as a cost).

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
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

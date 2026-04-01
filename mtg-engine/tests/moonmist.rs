//! Tests for Moonmist.
//!
//! Oracle: {1}{G} Instant
//! Transform all Humans. Prevent all combat damage that would be dealt this turn
//! by creatures other than Werewolves and Wolves.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// After Moonmist resolves, the flag is set.
#[test]
fn sets_prevention_flag() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let moonmist = castable_spell(&mut state, &reg, "Moonmist", P0);
    let new_state = cast_and_resolve(&state, &reg, moonmist, vec![]);

    assert!(new_state.prevent_non_wolf_werewolf_combat_damage,
        "Moonmist should set the prevention flag");
}

/// Non-Wolf/Werewolf creature deals no combat damage to player after Moonmist.
#[test]
fn prevents_non_wolf_combat_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.prevent_non_wolf_werewolf_combat_damage = true;
    state.combat = Some(mtg_engine::state::CombatState::new());

    // A plain 3/3 creature (not Wolf or Werewolf).
    let attacker = ready_creature(&mut state, P0, 3, 3);
    state.combat.as_mut().unwrap().attackers.insert(attacker, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.players[1].life, 20,
        "Non-Wolf creature should deal no combat damage after Moonmist");
}

/// Wolf creature still deals combat damage after Moonmist.
#[test]
fn wolf_still_deals_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.prevent_non_wolf_werewolf_combat_damage = true;
    state.combat = Some(mtg_engine::state::CombatState::new());

    // Use a named Wolf card.
    let wolf = named_creature(&mut state, &reg, "Darkthicket Wolf", P0);
    state.combat.as_mut().unwrap().attackers.insert(wolf, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert!(state.players[1].life < 20,
        "Wolf creature should still deal combat damage after Moonmist");
}

/// Non-Wolf creature deals no combat damage to creatures after Moonmist.
#[test]
fn prevents_non_wolf_combat_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.prevent_non_wolf_werewolf_combat_damage = true;
    state.combat = Some(mtg_engine::state::CombatState::new());

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.combat.as_mut().unwrap().attackers.insert(attacker, P1);
    state.combat.as_mut().unwrap().blocker_assignments.insert(blocker, vec![attacker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // Neither creature should have taken damage (both are non-Wolf/Werewolf).
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "Attacker should take no damage when blocker's damage is prevented");
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "Blocker should take no damage when attacker's damage is prevented");
}

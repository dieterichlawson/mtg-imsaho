//! Tests for Geist of Saint Traft.
//!
//! Oracle: {1}{W}{U} 2/2 Legendary Creature — Spirit Cleric
//! Hexproof
//! Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token
//! with flying that's tapped and attacking. Exile that token at end of combat.

mod common;

use common::*;
use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

#[test]
fn geist_creates_angel_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);

    // Simulate attack trigger.
    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(geist, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });
    behavior.on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    // Should have created an Angel token on the battlefield, tapped and attacking.
    let angel = state.objects.values()
        .find(|o| o.name == "Angel" && o.zone == Zone::Battlefield)
        .expect("Angel token should be on the battlefield");
    assert_eq!(angel.power, Some(4));
    assert_eq!(angel.toughness, Some(4));
    assert!(angel.tapped, "Angel should be tapped");
}

#[test]
fn angel_exiled_at_end_of_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(geist, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });

    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    let angel_id = state.objects.values()
        .find(|o| o.name == "Angel" && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("Angel should exist");

    // End combat fires the delayed trigger; auto-resolve exiles the Angel.
    state.step = Step::EndCombat;
    fire_step_trigger(&mut state, Step::EndCombat, &reg);

    assert_eq!(state.get_object(angel_id).unwrap().zone, Zone::Exile,
        "Angel token should be exiled at end of combat");
}

#[test]
fn angel_exiled_even_if_geist_dies() {
    // "Exile that token at end of combat" is a delayed triggered ability.
    // It fires even if Geist has left the battlefield.
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(geist, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });

    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    let angel_id = state.objects.values()
        .find(|o| o.name == "Angel" && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("Angel should exist");

    // Kill the Geist before end of combat.
    state.move_object(geist, Zone::Graveyard, &reg);
    assert_eq!(state.get_object(geist).unwrap().zone, Zone::Graveyard,
        "Geist should be dead");

    // End combat fires the delayed trigger; the exile entry was recorded
    // at attack time, so the Angel is still exiled even after Geist dies.
    state.step = Step::EndCombat;
    fire_step_trigger(&mut state, Step::EndCombat, &reg);

    assert_eq!(state.get_object(angel_id).unwrap().zone, Zone::Exile,
        "Angel should be exiled even when Geist has left the battlefield");
}

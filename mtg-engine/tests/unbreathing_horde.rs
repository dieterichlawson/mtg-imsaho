//! Tests for Unbreathing Horde.
//!
//! Oracle: {2}{B} 0/0 Zombie
//! Unbreathing Horde enters the battlefield with a +1/+1 counter on it for each
//! other Zombie you control and each Zombie card in your graveyard.
//! If Unbreathing Horde would be dealt damage, prevent that damage and remove a
//! +1/+1 counter from it.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Combat damage is prevented and a counter is removed.
#[test]
fn prevents_combat_damage_removes_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    // Give it 3 +1/+1 counters.
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    // Attacker attacks, Horde blocks.
    let attacker = ready_creature(&mut state, P1, 2, 2);
    state.combat.as_mut().unwrap().attackers.insert(attacker, P0);
    // blocker_assignments maps attacker -> [blockers]
    state.combat.as_mut().unwrap().blocker_assignments.insert(attacker, vec![horde]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The Horde should have taken no damage but lost a counter.
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Damage should be prevented");
    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 2, "Should have lost one +1/+1 counter");
}

/// When Unbreathing Horde deals damage as attacker, the other creature still takes damage.
#[test]
fn still_deals_damage_to_others() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    let blocker = ready_creature(&mut state, P1, 2, 5);
    // Horde attacks, blocker blocks.
    state.combat.as_mut().unwrap().attackers.insert(horde, P1);
    state.combat.as_mut().unwrap().blocker_assignments.insert(horde, vec![blocker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The blocker should have taken damage from Horde (0 base + 3 counters = 3 power).
    assert!(state.get_object(blocker).unwrap().damage_marked > 0,
        "Blocker should take damage from Unbreathing Horde");
    // The Horde should have taken no damage (prevented).
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Horde damage should be prevented");
}

/// ETB counter count is correct with zombies on battlefield and graveyard.
#[test]
fn enters_with_correct_counter_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 2 zombies on battlefield.
    let _z1 = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let _z2 = named_creature(&mut state, &reg, "Diregraf Ghoul", P0);

    // Put 1 zombie in graveyard.
    let z3 = named_creature(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(z3, Zone::Graveyard);

    // Place Unbreathing Horde on battlefield and trigger ETB manually.
    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    let behavior = reg.get(state.get_object(horde).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, horde, &reg);

    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    // 2 battlefield zombies + 1 graveyard zombie = 3 counters.
    assert_eq!(counters, 3, "Should have 3 +1/+1 counters (2 bf + 1 gy zombies)");
}

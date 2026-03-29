//! Tests for APNAP (Active Player, Non-Active Player) trigger ordering.
//!
//! CR 603.3b: When multiple triggered abilities trigger simultaneously,
//! the active player puts their triggers on the stack first (resolving last),
//! then the non-active player puts theirs on top (resolving first).
//!
//! The current trigger system is synchronous (not stack-based), so we
//! implement a partial fix: active player's triggers process first within
//! each event. This determines which effects modify game state first when
//! triggers interact.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ════════════════════════════════════════════════════════════════════
// Test: Active player's triggers should process before non-active player's
//
// P0 (active) has Rage Thrower (deals 2 to opponent when creature dies).
// P1 (non-active) has Falkenrath Noble (drains 1 when own creature dies).
// P0's creature dies.
//
// APNAP order: P0's Rage Thrower trigger fires first, then P1's Noble.
// We verify by checking the order of LifeChanged events.
// ════════════════════════════════════════════════════════════════════

#[test]
fn active_player_triggers_fire_first() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // P0 is the active player.

    // Create P1's trigger FIRST so it gets a lower ObjectId.
    // If the engine iterates by ObjectId order (or HashMap insertion order),
    // this would cause P1's trigger to fire first — which is wrong per APNAP.
    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P1);

    // P0 has Rage Thrower (triggers on any creature death).
    let _thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);

    // P1's creature that will die.
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // Record life totals.
    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    // Kill the creature via SBAs.
    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Process triggers — both Rage Thrower and Falkenrath Noble should fire.
    triggers::process_triggers(&mut state, &reg);

    // Rage Thrower (P0) should deal 2 damage to P1.
    // Falkenrath Noble (P1) should drain 1 from P0 (opponent of Noble's controller).
    assert_eq!(state.get_player(P0).life, p0_life_before - 1,
        "Falkenrath Noble should drain 1 from P0");
    assert_eq!(state.get_player(P1).life, p1_life_before - 2 + 1,
        "Rage Thrower deals 2 to P1, Noble gains 1 for P1");

    // Check that P0's trigger (Rage Thrower) fired BEFORE P1's (Noble)
    // by examining the order of LifeChanged events.
    let life_events: Vec<_> = state.events.iter()
        .filter_map(|e| {
            if let GameEvent::LifeChanged { player, old, new_life } = e {
                Some((*player, *old, *new_life))
            } else {
                None
            }
        })
        .collect();

    // Expected APNAP order (active player's triggers first):
    // 1. Rage Thrower (P0's trigger): P1 loses 2 life
    // 2. Falkenrath Noble (P1's trigger): P0 loses 1, P1 gains 1
    //
    // So the first LifeChanged should be P1 losing life (from Rage Thrower).
    assert!(!life_events.is_empty(), "Should have life change events");
    let (first_player, first_old, first_new) = life_events[0];
    assert_eq!(first_player, P1,
        "APNAP: Active player's trigger (Rage Thrower → P1 loses life) should fire first. \
         Events: {:?}", life_events);
    assert!(first_new < first_old,
        "First event should be P1 losing life from Rage Thrower");
}

// ════════════════════════════════════════════════════════════════════
// Test: Triggers from the same player process in order
//
// P0 has both Rage Thrower and Unruly Mob. P0's creature dies.
// Both trigger. Both should fire (order among same player's triggers
// doesn't matter for APNAP, but they should all fire).
// ════════════════════════════════════════════════════════════════════

#[test]
fn same_player_multiple_triggers_all_fire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);
    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);

    // P0's creature dies.
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    let p1_life_before = state.get_player(P1).life;

    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    // Rage Thrower should deal 2 to P1.
    assert_eq!(state.get_player(P1).life, p1_life_before - 2,
        "Rage Thrower should deal 2 damage to P1");

    // Unruly Mob should gain a +1/+1 counter.
    let counters = state.get_counter_count(mob, CounterType::PlusOnePlusOne);
    assert_eq!(counters, 1,
        "Unruly Mob should gain a +1/+1 counter when ally dies");
}

// ════════════════════════════════════════════════════════════════════
// Test: Order matters when trigger can kill a creature
//
// This is the scenario where APNAP ordering has real gameplay impact.
// P0 (active) has Rage Thrower. P1 has a 2/2 Falkenrath Noble.
// A creature dies. In APNAP order, P0's Rage Thrower fires first
// and deals 2 damage to P1. Then P1's Noble triggers.
//
// If Rage Thrower instead targeted Noble (not the player), it could
// kill Noble before Noble's trigger fires. But our Rage Thrower
// targets the player, so this specific scenario doesn't kill Noble.
//
// We can still test: P0's trigger modifies P1's life, and P1's trigger
// should see P1's UPDATED life (after Rage Thrower's damage).
// ════════════════════════════════════════════════════════════════════

#[test]
fn apnap_order_active_player_modifies_state_first() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create P1's trigger FIRST (lower ObjectId) to expose ordering bugs.
    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P1);

    // P0 (active) has Rage Thrower.
    let _thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);

    // P1 controls a creature that will die.
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // P1 starts at exactly 2 life.
    state.get_player_mut(P1).life = 2;

    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    // In APNAP order: Rage Thrower (P0) fires first, dealing 2 to P1.
    // P1 goes to 0 life. Then Noble (P1) fires, P1 gains 1 → P1 at 1.
    // P0 loses 1 from Noble → P0 at 19.
    //
    // If Noble fired first: P1 gains 1 → 3, P0 loses 1 → 19.
    // Then Rage Thrower: P1 loses 2 → 1. P0 still at 19.
    // Same final state in this case, but the intermediate state differs.
    //
    // Final state should be the same either way here, but the EVENT ORDER
    // should reflect APNAP: Rage Thrower's event before Noble's event.
    assert_eq!(state.get_player(P1).life, 1,
        "P1 should be at 1 life (started at 2, -2 from Thrower, +1 from Noble)");
    assert_eq!(state.get_player(P0).life, 19,
        "P0 should be at 19 life (-1 from Noble)");

    // Verify event order: first LifeChanged should be from Rage Thrower (P1 losing life).
    let life_events: Vec<_> = state.events.iter()
        .filter_map(|e| {
            if let GameEvent::LifeChanged { player, old, new_life } = e {
                Some((*player, *old, *new_life))
            } else {
                None
            }
        })
        .collect();

    assert!(life_events.len() >= 3,
        "Should have at least 3 life change events (Thrower→P1, Noble→P0, Noble→P1). Got: {:?}", life_events);

    // First event should be P1 losing 2 life (Rage Thrower, active player's trigger).
    let (first_player, _, _) = life_events[0];
    assert_eq!(first_player, P1,
        "APNAP: First life change should be from active player's Rage Thrower targeting P1. \
         Events: {:?}", life_events);
}

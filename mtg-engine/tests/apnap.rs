//! Tests for APNAP (Active Player, Non-Active Player) trigger ordering
//! with stack-based triggered abilities.
//!
//! CR 603.3b: When multiple triggered abilities trigger simultaneously,
//! the active player puts their triggers on the stack first (bottom),
//! then the non-active player puts theirs on top (top). Since the stack
//! resolves LIFO, the non-active player's triggers resolve FIRST.
//!
//! CR 603.3: Triggered abilities go on the stack. They are not resolved
//! immediately — players get priority to respond with instants.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ════════════════════════════════════════════════════════════════════
// APNAP resolution order: non-active player's triggers resolve first
//
// P0 (active) has Rage Thrower (deals 2 on any death).
// P1 (non-active) has Falkenrath Noble (drains 1 on own creature death).
// P1's creature dies. Both trigger.
//
// Stack (bottom to top): [Rage Thrower trigger, Noble trigger]
// Resolution (LIFO): Noble resolves first, then Rage Thrower.
//
// We verify by checking the order of LifeChanged events.
// Noble's drain (P0 loses 1, P1 gains 1) should appear BEFORE
// Rage Thrower's damage (P1 loses 2).
// ════════════════════════════════════════════════════════════════════

#[test]
fn non_active_player_triggers_resolve_first() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // P0 is the active player.

    // Create P1's trigger first (lower ObjectId) to ensure the test
    // catches ordering bugs — HashMap iteration would process P1 first.
    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P1);
    let _thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);

    // P1's creature dies. Both triggers fire.
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    // Both effects should have happened.
    // Noble (P1's): P0 loses 1, P1 gains 1.
    // Thrower (P0's): P1 loses 2.
    assert_eq!(state.get_player(P0).life, p0_life_before - 1,
        "P0 should lose 1 from Noble");
    assert_eq!(state.get_player(P1).life, p1_life_before + 1 - 2,
        "P1 should gain 1 from Noble and lose 2 from Thrower");

    // Check resolution order via events.
    // APNAP with LIFO: NAP's trigger (Noble) resolves first.
    // First LifeChanged events should be from Noble (P0 loses 1, P1 gains 1),
    // then from Thrower (P1 loses 2).
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
        "Should have at least 3 life events. Got: {:?}", life_events);

    // Noble's events come first (NAP resolves first in LIFO).
    // Noble drains: P0 loses 1 life. That should be the first LifeChanged.
    let (first_player, first_old, first_new) = life_events[0];
    assert_eq!(first_player, P0,
        "APNAP LIFO: Non-active player's Noble trigger should resolve first, \
         causing P0 to lose 1 life. Events: {:?}", life_events);
    assert_eq!(first_new, first_old - 1,
        "First event should be P0 losing 1 life from Noble's drain");
}

// ════════════════════════════════════════════════════════════════════
// Same player's multiple triggers all fire
// ════════════════════════════════════════════════════════════════════

#[test]
fn same_player_multiple_triggers_all_fire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);
    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);

    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    let p1_life_before = state.get_player(P1).life;

    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, p1_life_before - 2,
        "Rage Thrower should deal 2 damage to P1");

    let counters = state.get_counter_count(mob, CounterType::PlusOnePlusOne);
    assert_eq!(counters, 1,
        "Unruly Mob should gain a +1/+1 counter when ally dies");
}

// ════════════════════════════════════════════════════════════════════
// APNAP with life at stake
//
// P1 at 2 life. P0 has Rage Thrower, P1 has Falkenrath Noble.
// P1's creature dies. With correct LIFO ordering:
// 1. Noble resolves first: P1 gains 1 (→3), P0 loses 1 (→19)
// 2. Thrower resolves: P1 loses 2 (→1)
// Final: P0=19, P1=1
//
// With wrong (AP-first) ordering:
// 1. Thrower: P1 loses 2 (→0) — but game isn't over yet mid-trigger
// 2. Noble: P1 gains 1 (→1), P0 loses 1 (→19)
// Final: P0=19, P1=1 (same in this case, but intermediate state differs)
// ════════════════════════════════════════════════════════════════════

#[test]
fn apnap_lifo_order_with_life_totals() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P1);
    let _thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);

    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    state.get_player_mut(P1).life = 2;

    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    // With LIFO: Noble first (P1: 2→3), then Thrower (P1: 3→1).
    assert_eq!(state.get_player(P1).life, 1,
        "P1 should be at 1 (started at 2, Noble +1=3, Thrower -2=1)");
    assert_eq!(state.get_player(P0).life, 19,
        "P0 should be at 19 (Noble -1)");

    // Verify the intermediate state was correct: Noble resolved first.
    let life_events: Vec<_> = state.events.iter()
        .filter_map(|e| {
            if let GameEvent::LifeChanged { player, old, new_life } = e {
                Some((*player, *old, *new_life))
            } else {
                None
            }
        })
        .collect();

    // Noble fires first (LIFO): P0 loses 1 (20→19), P1 gains 1 (2→3).
    // Then Thrower: P1 loses 2 (3→1).
    // So the first LifeChanged should show P0 going from 20 to 19.
    assert!(!life_events.is_empty());
    assert_eq!(life_events[0], (P0, 20, 19),
        "First event should be Noble draining P0 (20→19). Events: {:?}", life_events);
}

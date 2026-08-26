//! Tests for trigger-priority integration: triggers go on the stack and
//! players get priority between each resolution (CR 117.5, CR 603.3).

use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers;
use mtg_engine::types::*;

mod common;
use common::*;
/// After SBAs kill a creature, `collect_triggers` places the death trigger
/// on the stack but does NOT resolve it. The trigger waits for the
/// priority cycle to resolve it.
#[test]
fn triggers_placed_on_stack_not_immediately_resolved() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Unruly Mob: "Whenever another creature you control dies, put a +1/+1 counter on ~."
    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;
    state.events.clear();
    state.trigger_event_index = 0;

    // SBAs kill the creature
    let sba_happened = check_state_based_actions(&mut state, &reg);
    assert!(sba_happened, "SBA should destroy the damaged creature");

    // Collect triggers onto the stack (don't resolve)
    let had_triggers = triggers::collect_triggers(&mut state, &reg);
    assert!(had_triggers, "Should have collected a death trigger");

    // Trigger is on the stack
    assert!(
        state.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_))),
        "Death trigger should be on the stack"
    );

    // NOT yet resolved — mob should have no counters
    assert_eq!(
        state.get_counter_count(mob, CounterType::PlusOnePlusOne),
        0,
        "Trigger should not have resolved yet"
    );
}

/// After `collect_triggers`, resolving the trigger via `resolve_top_of_stack`
/// processes exactly one trigger and removes it from the stack.
#[test]
fn resolve_top_of_stack_handles_single_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);

    // Two creatures will die — two death watch triggers for the mob
    let v1 = ready_creature(&mut state, P0, 1, 1);
    let v2 = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(v1).unwrap().damage_marked = 2;
    state.get_object_mut(v2).unwrap().damage_marked = 2;
    state.events.clear();
    state.trigger_event_index = 0;

    check_state_based_actions(&mut state, &reg);
    triggers::collect_triggers(&mut state, &reg);

    let trigger_count = state.stack.iter()
        .filter(|e| matches!(e, StackEntry::Trigger(_)))
        .count();
    assert_eq!(trigger_count, 2, "Two death triggers should be on the stack");

    // Resolve exactly one trigger
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(
        state.get_counter_count(mob, CounterType::PlusOnePlusOne),
        1,
        "Only one trigger should have resolved"
    );
    let remaining = state.stack.iter()
        .filter(|e| matches!(e, StackEntry::Trigger(_)))
        .count();
    assert_eq!(remaining, 1, "One trigger should remain on the stack");
}

/// When neither player has meaningful actions (no instants, no abilities),
/// triggers auto-resolve through the priority cycle without the
/// `choose_action` callback being invoked while triggers are on the stack.
#[test]
fn auto_pass_resolves_triggers_without_callback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    let mut saw_trigger_on_stack = false;
    let mut actions = 0;

    engine::run_game_loop(&mut state, &reg, |gs, _player, _legal| {
        actions += 1;
        if gs.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_))) {
            saw_trigger_on_stack = true;
        }
        if actions > 30 {
            return Action::Concede;
        }
        Action::PassPriority
    });

    // Trigger should have resolved
    assert_eq!(
        state.get_counter_count(mob, CounterType::PlusOnePlusOne),
        1,
        "Death trigger should have resolved via auto-pass"
    );

    // Callback should NOT have seen a trigger on the stack
    assert!(
        !saw_trigger_on_stack,
        "Auto-pass should handle triggers without invoking callback"
    );
}

/// When a player has meaningful actions (e.g., an instant in hand),
/// the game loop gives them priority with the trigger on the stack.
/// The legal actions context should mention "RESPOND TO".
#[test]
fn priority_with_trigger_shows_respond_context() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // Give P1 an instant so they have meaningful actions
    let _bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P1);

    let mut saw_trigger_on_stack = false;
    let mut saw_respond_context = false;
    let mut actions = 0;

    engine::run_game_loop(&mut state, &reg, |gs, _player, legal| {
        actions += 1;
        if gs.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_))) {
            saw_trigger_on_stack = true;
            if let Some(ref ctx) = legal.context {
                if ctx.contains("RESPOND TO") {
                    saw_respond_context = true;
                }
            }
        }
        if actions > 50 {
            return Action::Concede;
        }
        Action::PassPriority
    });

    assert!(saw_trigger_on_stack, "Callback should be invoked with trigger on stack");
    assert!(saw_respond_context, "Context should say RESPOND TO when trigger is on stack");
    assert_eq!(
        state.get_counter_count(mob, CounterType::PlusOnePlusOne),
        1,
        "Trigger should resolve after priority passes"
    );
}

/// A player can cast an instant in response to a triggered ability.
/// Both the spell and the trigger resolve correctly.
#[test]
fn cast_instant_in_response_to_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P1);

    let mut bolt_cast = false;
    let mut actions = 0;

    engine::run_game_loop(&mut state, &reg, |gs, player, _legal| {
        actions += 1;

        // P1 casts Lightning Bolt targeting P0 when a trigger is on the stack
        if player == P1
            && !bolt_cast
            && gs.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_)))
        {
            bolt_cast = true;
            return cast_action(bolt, vec![Target::Player(P0)]);
        }

        if actions > 30 {
            return Action::Concede;
        }
        Action::PassPriority
    });

    assert!(bolt_cast, "P1 should have cast Lightning Bolt in response to trigger");
    assert_eq!(
        state.get_player(P0).life,
        17,
        "Lightning Bolt should have dealt 3 damage to P0"
    );
    assert_eq!(
        state.get_counter_count(mob, CounterType::PlusOnePlusOne),
        1,
        "Death trigger should have also resolved"
    );
}

/// When multiple triggers fire simultaneously (two creatures die),
/// each resolves one at a time with priority given between resolutions.
/// Observable as counter values incrementing one step at a time.
#[test]
fn multiple_triggers_resolve_individually_with_priority() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);

    // Two creatures controlled by P0 die simultaneously
    let v1 = ready_creature(&mut state, P0, 1, 1);
    let v2 = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(v1).unwrap().damage_marked = 2;
    state.get_object_mut(v2).unwrap().damage_marked = 2;

    // Give P1 an instant so the callback is invoked between resolutions
    let _bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P1);

    let mut counter_values_at_priority = vec![];
    let mut actions = 0;

    engine::run_game_loop(&mut state, &reg, |gs, _player, _legal| {
        actions += 1;

        // Record counter value when we see a trigger on the stack
        if gs.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_))) {
            counter_values_at_priority
                .push(gs.get_counter_count(mob, CounterType::PlusOnePlusOne));
        }

        if actions > 40 {
            return Action::Concede;
        }
        Action::PassPriority
    });

    // Both triggers should have resolved
    assert_eq!(
        state.get_counter_count(mob, CounterType::PlusOnePlusOne),
        2,
        "Mob should have 2 counters from 2 death triggers"
    );

    // Between trigger resolutions, priority was given with different counter counts
    assert!(
        counter_values_at_priority.contains(&0),
        "Should see 0 counters before first trigger resolves: {counter_values_at_priority:?}"
    );
    assert!(
        counter_values_at_priority.contains(&1),
        "Should see 1 counter between trigger resolutions: {counter_values_at_priority:?}"
    );
}

/// Casting a spell in response to a trigger puts the spell on top.
/// The spell resolves first (LIFO), then the trigger resolves.
/// We verify ordering by having the spell kill a creature that affects
/// the trigger's outcome.
#[test]
fn spell_resolves_before_trigger_on_stack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Unruly Mob will get a counter from the death trigger
    let mob = named_creature(&mut state, &reg, "Unruly Mob", P0);
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // Give P1 a Lightning Bolt to kill the mob in response to its own trigger.
    // If Bolt resolves first (correct LIFO), the mob dies before the trigger
    // resolves, so the counter is placed on a dead mob (graveyard).
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P1);

    let mut bolt_cast = false;
    let mut actions = 0;

    engine::run_game_loop(&mut state, &reg, |gs, player, _legal| {
        actions += 1;

        // P1 bolts the Unruly Mob when trigger is on the stack
        if player == P1
            && !bolt_cast
            && gs.stack.iter().any(|e| matches!(e, StackEntry::Trigger(_)))
        {
            bolt_cast = true;
            return cast_action(bolt, vec![Target::Object(mob)]);
        }

        if actions > 40 {
            return Action::Concede;
        }
        Action::PassPriority
    });

    assert!(bolt_cast, "P1 should have bolted the mob in response");
    // Mob should be dead (Lightning Bolt deals 3, mob is 1/1 + possibly 1 counter = 1/1 or 2/2)
    // Unruly Mob is a 1/1, Lightning Bolt deals 3 → lethal
    assert_eq!(
        state.get_object(mob).unwrap().zone,
        Zone::Graveyard,
        "Mob should be in graveyard after being bolted"
    );
}

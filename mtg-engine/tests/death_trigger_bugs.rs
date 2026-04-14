//! Tests for death trigger bugs:
//! 1. Spurious death-watch triggers from permanents without AnyCreatureDies triggers
//! 2. Duplicate death log entries

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::ids::PlayerId;
use mtg_engine::types::{Step, Zone};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// Lands and other permanents without death triggers should NOT generate
/// triggered abilities when a creature dies.
#[test]
fn lands_should_not_trigger_on_creature_death() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a Swamp on the battlefield (has no death triggers).
    let swamp_id = registry.get_id_by_name("Swamp").unwrap();
    state.create_object(swamp_id, P0, Zone::Battlefield, None, None);

    // Put a Falkenrath Noble on the battlefield (HAS a death trigger).
    let _noble = named_creature(&mut state, &registry, "Falkenrath Noble", P0);

    // Put an opponent creature that will die.
    let victim = ready_creature(&mut state, P1, 1, 1);

    // Kill the victim via combat damage.
    state.get_object_mut(victim).unwrap().damage_marked = 5;
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // Verify the CreatureDied event was generated.
    let death_events: Vec<_> = state.events.iter().filter(|e|
        matches!(e, mtg_engine::events::GameEvent::CreatureDied { .. })
    ).collect();
    assert!(!death_events.is_empty(), "Expected at least one CreatureDied event");

    // Process triggers.
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // Check the stack and awaiting_action: should only have triggers from
    // Falkenrath Noble (which has AnyCreatureDies), NOT from Swamp.
    let stack_names: Vec<String> = state.stack.iter().map(|entry| {
        match entry {
            mtg_engine::state::StackEntry::Trigger(t) => t.display_name(&registry),
            mtg_engine::state::StackEntry::Spell(id) =>
                state.get_object(*id).map_or("?".into(), |o| o.name.clone()),
        }
    }).collect();

    for name in &stack_names {
        assert!(!name.contains("Swamp"),
            "Swamp should not have a triggered ability on creature death, but found: {name}");
    }

    // Falkenrath Noble's trigger should have fired — either still on the stack
    // or already resolving (awaiting target choice).
    let has_noble = stack_names.iter().any(|n| n.contains("Falkenrath Noble"))
        || state.awaiting_action.is_some();
    assert!(has_noble,
        "Falkenrath Noble should have a death trigger, stack: {:?}, awaiting: {:?}",
        stack_names, state.awaiting_action);
}

/// When a creature dies from combat damage, the death should be logged
/// exactly once, not twice.
#[test]
fn creature_death_logged_once() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    // Simulate combat: they trade.
    combat::declare_attackers(&mut state, &[(attacker, P1)], &registry);
    combat::declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &registry);
    combat::deal_combat_damage(&mut state, &registry);
    // SBAs kill both creatures.
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // Count how many "died" log entries there are.
    let death_logs: Vec<_> = state.game_log.iter()
        .filter(|e| e.message.contains("died"))
        .collect();

    // There should be exactly 2 (one per creature), not 4.
    assert_eq!(death_logs.len(), 2,
        "Expected 2 death log entries (one per creature), got {}: {:?}",
        death_logs.len(), death_logs.iter().map(|e| &e.message).collect::<Vec<_>>());
}

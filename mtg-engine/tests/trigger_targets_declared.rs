//! CR 603.3b: a triggered ability's targets are chosen when the trigger is put
//! onto the stack, not when it resolves.
//!
//! Four cards declared `target_requirement: None` and then built their own
//! target prompt inside the resolution handler. The end result usually looked
//! right, but it broke three rules at once:
//!
//! - **603.3b** — the controller got to watch opponents respond to the trigger
//!   before committing to a target.
//! - **603.3c** — a trigger with no legal targets must never reach the stack.
//!   Since it went on undeclared, that check never ran and the handler quietly
//!   did nothing, leaving a spurious priority window behind.
//! - **608.2b** — the resolution-time legality re-check only runs when the
//!   trigger carries declared targets, so it was skipped too.
//!
//! Declaring the requirement lets the engine's existing
//! `process_pending_trigger_pushes` do all three.

mod common;

use common::*;
use mtg_engine::cards::TriggerKind;
use mtg_engine::state::GameState;
use mtg_engine::triggers::{self, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;
/// Every card whose triggered ability targets must say so, or the engine
/// cannot choose the target at the right time.
#[test]
fn targeted_triggers_declare_a_target_requirement() {
    let reg = registry();
    let cards = [
        ("Elder Cathar", TriggerKind::SelfDies),
        ("Bloodgift Demon", TriggerKind::Upkeep),
        ("Selhoff Occultist", TriggerKind::SelfDies),
        ("Selhoff Occultist", TriggerKind::AnyCreatureDies),
    ];
    // Curse of the Pierced Heart is deliberately absent: "deals 1 damage to
    // enchanted player" does not target. Its ticket is about WHEN the trigger
    // fires, covered by `curse_only_triggers_on_the_enchanted_players_upkeep`.
    for (name, kind) in cards {
        let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown {name}"));
        let ability = reg.get(card_id).unwrap().card_data().triggered_abilities
            .into_iter()
            .find(|t| t.kind == kind)
            .unwrap_or_else(|| panic!("{name} should declare a {kind:?} trigger"));
        assert!(ability.target_requirement.is_some(),
            "{name}'s {kind:?} ability targets, so it must declare a \
             target_requirement — otherwise the engine pushes it untargeted \
             and the card has to pick at resolution (CR 603.3b)");
    }
}

/// CR 603.3c: with no legal target, the ability never reaches the stack.
#[test]
fn elder_cathar_trigger_is_removed_with_no_legal_targets() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elder Cathar dies with no other creature its controller controls.
    let cathar = named_creature(&mut state, &reg, "Elder Cathar", P0);
    named_creature(&mut state, &reg, "Walking Corpse", P1); // opponent's — not a legal target
    mtg_engine::destruction::try_destroy(&mut state, cathar, &reg);
    triggers::collect_triggers(&mut state, &reg);

    let on_stack = state.stack.iter()
        .filter(|e| matches!(e, mtg_engine::state::StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: dead_id, .. },
                event: TriggerEvent::SelfDies }) if *dead_id == cathar))
        .count();
    assert_eq!(on_stack, 0,
        "with no creature its controller controls, Elder Cathar's ability has \
         no legal target and must not go on the stack at all (CR 603.3c)");
}

/// The counter still lands on the sole legal target, auto-picked as the
/// trigger goes on the stack.
#[test]
fn elder_cathar_puts_counters_on_the_declared_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_creature(&mut state, &reg, "Elder Cathar", P0);
    let bear = named_creature(&mut state, &reg, "Walking Corpse", P0);
    mtg_engine::destruction::try_destroy(&mut state, cathar, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, bear, CounterType::PlusOnePlusOne), 1,
        "the non-Human Walking Corpse gets one counter");
}

/// "If that creature is a Human, put two instead" still applies through the
/// declared-target path.
#[test]
fn elder_cathar_human_bonus_survives_the_declared_target_path() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_creature(&mut state, &reg, "Elder Cathar", P0);
    let human = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    mtg_engine::destruction::try_destroy(&mut state, cathar, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, human, CounterType::PlusOnePlusOne), 2,
        "Avacyn's Pilgrim is a Human, so it gets two counters");
}

/// "At the beginning of ENCHANTED PLAYER's upkeep" fires only on that
/// player's upkeep. `TriggerScope` has no value for "a specific other
/// player", so the curse gates dispatch through `should_trigger` instead of
/// letting the trigger reach the stack and doing nothing.
#[test]
fn curse_only_triggers_on_the_enchanted_players_upkeep() {
    let reg = registry();

    for (active, should_fire) in [(P1, true), (P0, false)] {
        let mut state = game_at_step(Step::Upkeep, active);
        let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);

        state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
        triggers::collect_triggers(&mut state, &reg);

        let queued = count_upkeep_entries(&state, curse);
        assert_eq!(queued > 0, should_fire,
            "with p{} active, the curse on p1 should{} put its upkeep trigger \
             on the stack", active.0, if should_fire { "" } else { " NOT" });
    }
}

fn count_upkeep_entries(state: &GameState, object: mtg_engine::ids::ObjectId) -> usize {
    state.stack.iter()
        .filter(|e| matches!(e, mtg_engine::state::StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: object_id, .. },
                event: TriggerEvent::Upkeep }) if *object_id == object))
        .count()
}

//! Tests for Gutter Grime — dynamic Ooze token P/T.
//!
//! Oracle: "Whenever a nontoken creature you control dies, put a slime counter on
//! Gutter Grime, then create a green Ooze creature token with 'This creature's
//! power and toughness are each equal to the number of slime counters on Gutter Grime.'"
//!
//! Key behaviors:
//! - Ooze token P/T dynamically tracks slime counter count on source Gutter Grime
//! - Adding more slime counters makes ALL existing Ooze tokens bigger
//! - Only nontoken creatures trigger it
//! - Only creatures you control trigger it

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::triggers;
use mtg_engine::types::*;

/// When a nontoken creature dies, Gutter Grime should create an Ooze token
/// whose P/T dynamically equals the slime counter count.
#[test]
fn gutter_grime_creates_dynamic_pt_ooze() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Gutter Grime on the battlefield.
    let gutter_grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // Put a nontoken creature on the battlefield for P0.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().card_types = vec![CardType::Creature];

    // Simulate the creature dying.
    let card_id = state.get_object(creature).unwrap().card_id;
    state.move_object(creature, Zone::Graveyard, &reg);
    state.events.push(GameEvent::CreatureDied {
        object: creature,
        card_id,
        controller: P0,
        damaged_by: vec![],
        last_known_toughness: 2,
        is_token: false,
    });

    // Process triggers.
    triggers::process_triggers(&mut state, &reg);

    // Gutter Grime should have 1 slime counter.
    assert_eq!(counters_of(&state, gutter_grime, CounterType::Slime), 1,
        "Gutter Grime should have 1 slime counter");

    // Find the Ooze token.
    assert_eq!(count_tokens_named(&state, "Ooze"), 1, "Should have created 1 Ooze token");
    let ooze_id = find_token_named(&state, "Ooze").unwrap();

    // The Ooze token should have dynamic P/T equal to slime counter count (1).
    assert_eq!(state.effective_power(ooze_id, &reg), Some(1),
        "Ooze effective power should be 1 (1 slime counter)");
    assert_eq!(state.effective_toughness(ooze_id, &reg), Some(1),
        "Ooze effective toughness should be 1 (1 slime counter)");
}

/// When more creatures die, all Ooze tokens should grow as slime counters increase.
#[test]
fn gutter_grime_ooze_tokens_grow_with_more_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // Kill first creature.
    let creature1 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature1).unwrap().card_types = vec![CardType::Creature];
    let card_id1 = state.get_object(creature1).unwrap().card_id;
    state.move_object(creature1, Zone::Graveyard, &reg);
    state.events.push(GameEvent::CreatureDied {
        object: creature1, card_id: card_id1, controller: P0,
        damaged_by: vec![], last_known_toughness: 2, is_token: false,
    });
    triggers::process_triggers(&mut state, &reg);

    // Clear events so the second death is processed cleanly.
    state.events.clear();
    state.trigger_event_index = 0;

    // Kill second creature.
    let creature2 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature2).unwrap().card_types = vec![CardType::Creature];
    let card_id2 = state.get_object(creature2).unwrap().card_id;
    state.move_object(creature2, Zone::Graveyard, &reg);
    state.events.push(GameEvent::CreatureDied {
        object: creature2, card_id: card_id2, controller: P0,
        damaged_by: vec![], last_known_toughness: 3, is_token: false,
    });
    triggers::process_triggers(&mut state, &reg);

    // Gutter Grime should have 2 slime counters now.
    assert_eq!(counters_of(&state, gutter_grime, CounterType::Slime), 2,
        "Gutter Grime should have 2 slime counters");

    // Find all Ooze tokens.
    assert_eq!(count_tokens_named(&state, "Ooze"), 2, "Should have 2 Ooze tokens");

    // ALL Ooze tokens should have effective P/T = 2 (both track current slime count).
    let ooze_ids: Vec<_> = state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield && o.name == "Ooze")
        .map(|o| o.id)
        .collect();
    for ooze_id in ooze_ids {
        assert_eq!(state.effective_power(ooze_id, &reg), Some(2),
            "All Ooze tokens should have effective power 2 (2 slime counters)");
        assert_eq!(state.effective_toughness(ooze_id, &reg), Some(2),
            "All Ooze tokens should have effective toughness 2 (2 slime counters)");
    }
}

/// Token creatures dying should NOT trigger Gutter Grime.
#[test]
fn gutter_grime_ignores_token_deaths() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // Create a token creature.
    let token_id = state.create_token("Spirit", P0, 1, 1,
        vec![Color::White], vec![CardType::Creature], vec![], &reg)[0];

    // Kill the token.
    let card_id = state.get_object(token_id).unwrap().card_id;
    state.move_object(token_id, Zone::Graveyard, &reg);
    state.events.push(GameEvent::CreatureDied {
        object: token_id, card_id, controller: P0,
        damaged_by: vec![], last_known_toughness: 1, is_token: true,
    });
    triggers::process_triggers(&mut state, &reg);

    // Gutter Grime should have 0 slime counters.
    assert_eq!(counters_of(&state, gutter_grime, CounterType::Slime), 0,
        "Gutter Grime should not trigger on token deaths");
}

/// Opponent's creatures dying should NOT trigger Gutter Grime.
#[test]
fn gutter_grime_ignores_opponent_deaths() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // Create opponent's creature.
    let opp_creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(opp_creature).unwrap().card_types = vec![CardType::Creature];
    let card_id = state.get_object(opp_creature).unwrap().card_id;
    state.move_object(opp_creature, Zone::Graveyard, &reg);
    state.events.push(GameEvent::CreatureDied {
        object: opp_creature, card_id, controller: P1,
        damaged_by: vec![], last_known_toughness: 2, is_token: false,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, gutter_grime, CounterType::Slime), 0,
        "Gutter Grime should not trigger on opponent's creature deaths");
}

/// If Gutter Grime leaves the battlefield, existing Ooze tokens should become 0/0.
#[test]
fn gutter_grime_ooze_tokens_become_zero_without_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gutter_grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // Kill a creature to create an Ooze.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().card_types = vec![CardType::Creature];
    let card_id = state.get_object(creature).unwrap().card_id;
    state.move_object(creature, Zone::Graveyard, &reg);
    state.events.push(GameEvent::CreatureDied {
        object: creature, card_id, controller: P0,
        damaged_by: vec![], last_known_toughness: 2, is_token: false,
    });
    triggers::process_triggers(&mut state, &reg);

    let ooze_id = find_token_named(&state, "Ooze").unwrap();

    // Remove Gutter Grime from battlefield.
    state.move_object(gutter_grime, Zone::Graveyard, &reg);

    // The Ooze token should now have effective P/T = 0 (source has no slime counters visible).
    let eff_power = state.effective_power(ooze_id, &reg).unwrap();
    let eff_toughness = state.effective_toughness(ooze_id, &reg).unwrap();
    assert_eq!(eff_power, 0, "Ooze should be 0/0 when Gutter Grime leaves battlefield");
    assert_eq!(eff_toughness, 0, "Ooze should be 0/0 when Gutter Grime leaves battlefield");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug 99-001 (`audits/AUDIT_BUGS.md)`: Gutter Grime's `on_any_creature_dies`
/// checks `state.get_object(dead_id).is_token` to enforce the
/// "nontoken" oracle requirement. By the time this handler runs, SBA
/// 704.5d has already removed the dead token from `state.objects`, so
/// `state.get_object(dead_id)` returns None, `was_token` defaults to
/// false, and the handler proceeds to add a slime counter and create
/// an Ooze for *every* creature death — token or not.
///
/// Oracle (Gutter Grime): "Whenever a **nontoken** creature you
/// control dies, put a slime counter on this enchantment, then create
/// a green Ooze creature token..."
///
/// Failure mode: `gutter_grime.rs`. The dispatcher correctly
/// queues the trigger, the controller filter passes, then
/// `state.get_object(dead_id).map(|o| o.is_token).unwrap_or(false)`
/// returns `false` for an already-cleaned-up token. The fix needs the
/// dispatcher to thread `is_token` (or the dead `card_id`) into
/// `on_any_creature_dies` so the handler can check it from captured
/// state.
///
/// We simulate the post-cleanup state by passing a `dead_id` that's not
/// in `state.objects` and observing whether Gutter Grime's slime
/// counter was incremented.
#[test]
fn bug_99_001_gutter_grime_does_not_count_token_deaths() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &registry, "Gutter Grime", P0);
    let slime_before = state
        .get_object(grime)
        .unwrap()
        .counters
        .get(&CounterType::Slime)
        .copied()
        .unwrap_or(0);

    // The dead creature was a token that's already been cleaned up by
    // SBA 704.5d. Use an ObjectId that's not in state.objects.
    let dead_token_id = mtg_engine::ids::ObjectId(99999);
    assert!(
        state.get_object(dead_token_id).is_none(),
        "Test setup: dead_token_id should not be in state.objects"
    );

    let grime_card_id = state.get_object(grime).unwrap().card_id;
    let behavior = registry.get(grime_card_id).unwrap();
    behavior.on_any_creature_dies(
        &mut state,
        grime,
        dead_token_id,
        P0, // dead_controller (matches Gutter Grime's owner)
        &[],
        2, // dead_toughness
        true, // dead_is_token — this is the whole point of the bug
        &[],
        &registry,
    );

    let slime_after = state
        .get_object(grime)
        .unwrap()
        .counters
        .get(&CounterType::Slime)
        .copied()
        .unwrap_or(0);

    assert_eq!(
        slime_after, slime_before,
        "Gutter Grime should NOT add a slime counter when a TOKEN \
         creature dies (oracle says 'nontoken'). Bug 99-001: the \
         is_token check reads state.get_object(dead_id), but tokens \
         are already cleaned up by SBA 704.5d at trigger-resolution \
         time, so the handler treats them as nontoken. Slime counters \
         {slime_before} -> {slime_after}",
    );
}

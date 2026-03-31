//! Tests for the three Tier 3 engine systems: token creation,
//! +1/+1 counters, and triggered abilities (ETB, dies, death-watch).

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::state::GameState;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ══════════════════════════════════════════════════════════════════
// Token creation
// ══════════════════════════════════════════════════════════════════

/// Basic token creation puts a creature on the battlefield.
#[test]
fn token_created_on_battlefield() {
    let mut state = GameState::new(2);
    let token = state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
    );

    let obj = state.get_object(token).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert_eq!(obj.name, "Spirit");
    assert_eq!(obj.power, Some(1));
    assert_eq!(obj.toughness, Some(1));
    assert!(obj.is_token);
    assert!(obj.keywords.contains(&Keyword::Flying));
    assert!(obj.card_types.contains(&CardType::Creature));
    assert!(obj.summoning_sick, "Token should have summoning sickness");
    assert_eq!(obj.card_id, CardId(0), "Token should have sentinel CardId(0)");
}

/// Token has correct controller/owner.
#[test]
fn token_owned_by_creator() {
    let mut state = GameState::new(2);
    let token = state.create_token("Zombie", P1, 2, 2, vec![Color::Black], vec![CardType::Creature], vec![]);

    let obj = state.get_object(token).unwrap();
    assert_eq!(obj.owner, P1);
    assert_eq!(obj.controller, P1);
}

/// Tokens ceases to exist when moved off the battlefield (SBA).
#[test]
fn token_ceases_to_exist_when_killed() {
    let reg = registry();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    let token = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![]);

    // Kill the token (move to graveyard).
    state.move_object(token, Zone::Graveyard);

    // SBA should remove it entirely from the game.
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    assert!(state.get_object(token).is_none(),
        "Token should cease to exist after leaving the battlefield");
}

/// Token that is bounced (returned to hand) ceases to exist.
#[test]
fn token_ceases_to_exist_when_bounced() {
    let reg = registry();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    let token = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![]);

    state.move_object(token, Zone::Hand);

    check_state_based_actions_with_registry(&mut state, Some(&reg));
    assert!(state.get_object(token).is_none(),
        "Token should cease to exist when bounced to hand");
}

/// Multiple tokens can be created at once.
#[test]
fn multiple_tokens_created() {
    let mut state = GameState::new(2);
    let t1 = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying]);
    let t2 = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying]);

    assert_ne!(t1, t2, "Tokens should have unique IDs");
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P0).len(), 2);
}

/// Token shows up correctly in has_keyword.
#[test]
fn token_keyword_check_works() {
    let reg = registry();
    let mut state = GameState::new(2);
    let token = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying]);

    assert!(state.has_keyword(token, Keyword::Flying, &reg),
        "Token with flying keyword should be detected by has_keyword");
    assert!(!state.has_keyword(token, Keyword::Trample, &reg));
}

/// Token emits EnteredBattlefield event.
#[test]
fn token_creation_emits_etb_event() {
    let mut state = GameState::new(2);
    state.events.clear();
    let _token = state.create_token("Spirit", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);

    let has_etb = state.events.iter().any(|e| matches!(e, mtg_engine::events::GameEvent::EnteredBattlefield { .. }));
    assert!(has_etb, "Token creation should emit EnteredBattlefield event");
}

// ══════════════════════════════════════════════════════════════════
// +1/+1 Counters
// ══════════════════════════════════════════════════════════════════

/// Adding +1/+1 counters increases effective P/T.
#[test]
fn plus_one_counters_increase_pt() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.add_counters(creature, CounterType::PlusOnePlusOne, 3);

    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(5));
    assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), 3);
}

/// -1/-1 counters decrease effective P/T.
#[test]
fn minus_one_counters_decrease_pt() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    state.add_counters(creature, CounterType::MinusOneMinusOne, 2);

    assert_eq!(state.effective_power(creature, &reg), Some(1));
    assert_eq!(state.effective_toughness(creature, &reg), Some(1));
}

/// Creature with enough -1/-1 counters to reach 0 toughness dies to SBA.
#[test]
fn minus_counters_kill_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.add_counters(creature, CounterType::MinusOneMinusOne, 2);

    check_state_based_actions_with_registry(&mut state, Some(&reg));
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature with 0 effective toughness from counters should die");
}

/// Counters are cleared when leaving the battlefield.
#[test]
fn counters_cleared_on_zone_change() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.add_counters(creature, CounterType::PlusOnePlusOne, 5);

    state.move_object(creature, Zone::Graveyard);

    assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), 0,
        "Counters should be cleared when leaving the battlefield");
}

/// +1/+1 counters stack with aura bonuses.
#[test]
fn counters_stack_with_auras() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    // Attach Holy Strength (+1/+2).
    let hs = castable_spell(&mut state, &reg, "Holy Strength", P0);
    state = cast_and_resolve(&state, &reg, hs, vec![Target::Object(creature)]);

    // Add 2 +1/+1 counters.
    state.add_counters(creature, CounterType::PlusOnePlusOne, 2);

    // Base 2/2 + aura +1/+2 + counters +2/+2 = 5/6
    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(6));
}

// ══════════════════════════════════════════════════════════════════
// Triggered abilities
// ══════════════════════════════════════════════════════════════════

// Note: These test the trigger system infrastructure directly.
// Card-specific trigger tests are in the card test files.

/// EnteredBattlefield events are emitted when objects move to the battlefield.
#[test]
fn etb_event_emitted_on_zone_change() {
    let mut state = GameState::new(2);
    let id = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    state.events.clear();

    state.move_object(id, Zone::Battlefield);

    let has_etb = state.events.iter().any(|e|
        matches!(e, mtg_engine::events::GameEvent::EnteredBattlefield { object, .. } if *object == id)
    );
    assert!(has_etb, "Moving to battlefield should emit EnteredBattlefield");
}

/// ETB is NOT emitted when moving between non-battlefield zones.
#[test]
fn no_etb_for_non_battlefield_moves() {
    let mut state = GameState::new(2);
    let id = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    state.events.clear();

    state.move_object(id, Zone::Graveyard);

    let has_etb = state.events.iter().any(|e|
        matches!(e, mtg_engine::events::GameEvent::EnteredBattlefield { .. })
    );
    assert!(!has_etb, "Moving to graveyard should not emit ETB");
}

/// CreatureDied events trigger process_triggers (smoke test with no cards).
#[test]
fn trigger_processing_doesnt_crash_without_cards() {
    let reg = registry();
    let mut state = GameState::new(2);
    state.events.push(mtg_engine::events::GameEvent::CreatureDied {
        object: mtg_engine::ids::ObjectId(999),
        card_id: mtg_engine::ids::CardId(0),
        controller: mtg_engine::ids::PlayerId(0),
        damaged_by: Vec::new(),
        last_known_toughness: 0,
    });

    // Should not panic even with a nonexistent object.
    triggers::process_triggers(&mut state, &reg);
}

/// Tokens interact with triggers — a dying token emits CreatureDied.
#[test]
fn dying_token_emits_creature_died() {
    let reg = registry();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    let token = state.create_token("Spirit", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);
    state.get_object_mut(token).unwrap().damage_marked = 1;

    state.events.clear();
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    let died = state.events.iter().any(|e|
        matches!(e, mtg_engine::events::GameEvent::CreatureDied { object, .. } if *object == token)
    );
    assert!(died, "Token dying should emit CreatureDied");
}

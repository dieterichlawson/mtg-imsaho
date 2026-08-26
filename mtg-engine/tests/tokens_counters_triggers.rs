//! Tests for the three Tier 3 engine systems: token creation,
//! +1/+1 counters, and triggered abilities (ETB, dies, death-watch).

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::GameState;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Token creation
// ══════════════════════════════════════════════════════════════════

/// What `create_token` has to set up, since a token is built from arguments
/// rather than from a card: it is on the battlefield, under its creator's
/// control, with the characteristics it was given, summoning sick, and marked
/// as a token with the sentinel card id that means "no registry entry".
#[test]
fn a_created_token_has_everything_it_was_given() {
    let reg = registry();
    let mut state = GameState::new(2);

    let token = state.create_token(
        "Spirit", P1, 1, 1,
        vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];
    let obj = state.get_object(token).unwrap();

    assert_eq!(obj.zone, Zone::Battlefield);
    assert_eq!((obj.owner, obj.controller), (P1, P1), "owned and controlled by its creator");
    assert_eq!(obj.name, "Spirit");
    assert_eq!((obj.power, obj.toughness), (Some(1), Some(1)));
    assert!(obj.card_types.contains(&CardType::Creature));
    assert!(obj.keywords.contains(&Keyword::Flying));
    assert!(obj.is_token);
    assert!(obj.summoning_sick, "a token has summoning sickness like anything else");
    assert_eq!(obj.card_id, CardId(0), "the sentinel id meaning 'no registry entry'");

    // The keywords it was given are visible through the accessor, not just on
    // the object — a token has no card face for `has_keyword` to fall back to.
    assert!(state.has_keyword(token, Keyword::Flying, &reg));
    assert!(!state.has_keyword(token, Keyword::Trample, &reg));

    // A second token is a separate object.
    let other = state.create_token(
        "Spirit", P1, 1, 1,
        vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];
    assert_ne!(token, other, "each token is its own object");
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P1).len(), 2);
}

/// Tokens ceases to exist when moved off the battlefield (SBA).
/// CR 111.7 / 704.5d: a token that leaves the battlefield ceases to exist. It
/// touches the destination zone first — long enough for leaves-the-battlefield
/// and dies triggers to see it — and is then removed from the game by the next
/// SBA check, whichever zone it went to.
#[test]
fn a_token_that_leaves_the_battlefield_ceases_to_exist() {
    let reg = registry();
    for dest in [Zone::Graveyard, Zone::Hand, Zone::Exile, Zone::Library] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let token = state.create_token(
            "Spirit", P0, 1, 1,
            vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], &reg,
        )[0];

        state.move_object(token, dest, &reg);
        assert!(state.get_object(token).is_some(),
            "the token is still around in {dest:?} until SBAs run, so triggers can see it");

        check_state_based_actions(&mut state, &reg);
        assert!(state.get_object(token).is_none(),
            "a token that went to {dest:?} should have ceased to exist");
    }
}

/// The same rule reached the ordinary way — lethal damage rather than a direct
/// `move_object` — so the SBA that kills it and the SBA that removes it are
/// exercised in one pass.
#[test]
fn a_token_killed_by_damage_ceases_to_exist() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let token = state.create_token(
        "Zombie", P0, 2, 2, vec![], vec![CardType::Creature], vec![], &reg,
    )[0];
    state.get_object_mut(token).unwrap().summoning_sick = false;
    state.get_object_mut(token).unwrap().damage_marked = 3;

    check_state_based_actions(&mut state, &reg);

    assert!(state.get_object(token).is_none(),
        "a token killed by damage is gone, not sitting in the graveyard");
}

/// Multiple tokens can be created at once.
#[test]
fn multiple_tokens_created() {
    let reg = registry();
    let mut state = GameState::new(2);
    let t1 = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];
    let t2 = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];

    assert_ne!(t1, t2, "Tokens should have unique IDs");
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P0).len(), 2);
}

/// Token shows up correctly in `has_keyword`.
#[test]
fn token_keyword_check_works() {
    let reg = registry();
    let mut state = GameState::new(2);
    let token = state.create_token("Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], &reg)[0];

    assert!(state.has_keyword(token, Keyword::Flying, &reg),
        "Token with flying keyword should be detected by has_keyword");
    assert!(!state.has_keyword(token, Keyword::Trample, &reg));
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

    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature with 0 effective toughness from counters should die");
}

/// Counters are cleared when leaving the battlefield.
#[test]
fn counters_cleared_on_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.add_counters(creature, CounterType::PlusOnePlusOne, 5);

    state.move_object(creature, Zone::Graveyard, &reg);

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

/// `EnteredBattlefield` is emitted for arriving on the battlefield and for
/// nothing else — including for a token, which arrives without moving from
/// anywhere. Both arms, because "the event fired" alone is also true of an
/// engine that fires it on every move.
#[test]
fn entering_the_battlefield_is_announced_and_other_moves_are_not() {
    let reg = registry();
    let mut state = GameState::new(2);

    let entered = |state: &GameState, id: ObjectId| state.events.iter().any(|e|
        matches!(e, mtg_engine::events::GameEvent::EnteredBattlefield { object, .. } if *object == id));

    let card = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    state.events.clear();
    state.move_object(card, Zone::Battlefield, &reg);
    assert!(entered(&state, card), "a card arriving on the battlefield");

    let other = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    state.events.clear();
    state.move_object(other, Zone::Graveyard, &reg);
    assert!(!entered(&state, other), "hand to graveyard is not entering the battlefield");

    state.events.clear();
    let token = state.create_token("Spirit", P0, 1, 1, vec![], vec![CardType::Creature], vec![], &reg)[0];
    assert!(entered(&state, token),
        "a token arrives on the battlefield without moving from anywhere, and \
         still has to be announced — watchers see it like any other arrival");
}

/// A death event naming an object that does not exist must not panic. This
/// asserts nothing beyond that — it is a robustness check on the dispatch
/// loop's lookups, not a statement about any rule.
#[test]
fn trigger_processing_survives_an_event_about_a_missing_object() {
    let reg = registry();
    let mut state = GameState::new(2);
    state.events.push(mtg_engine::events::GameEvent::CreatureDied {
        object: mtg_engine::ids::ObjectId(999),
        card_id: mtg_engine::ids::CardId(0),
        controller: mtg_engine::ids::PlayerId(0),
        damaged_by: Vec::new(),
        last_known_toughness: 0,
        is_token: false,
    });

    // Should not panic even with a nonexistent object.
    triggers::process_triggers(&mut state, &reg);
}

/// Tokens interact with triggers — a dying token emits `CreatureDied`.
#[test]
fn dying_token_emits_creature_died() {
    let reg = registry();
    let mut state = GameState::new(2);
    let token = state.create_token("Spirit", P0, 1, 1, vec![], vec![CardType::Creature], vec![], &reg)[0];
    state.get_object_mut(token).unwrap().damage_marked = 1;

    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    let died = state.events.iter().any(|e|
        matches!(e, mtg_engine::events::GameEvent::CreatureDied { object, .. } if *object == token)
    );
    assert!(died, "Token dying should emit CreatureDied");
}

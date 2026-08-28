//! Cards that turn into something else: double-faced cards and the permanents
//! whose transform is driven by a trigger, an upkeep choice, or a cost.
//!
//! The rule itself is in `transform_dfc.rs` (CR 712 — one card, two faces, the
//! active face supplying every printed characteristic) and in
//! `werewolf_cards.rs` (the day/night trigger pair). This file is the
//! acceptance layer: does each card transform when its own text says it does,
//! and does what it becomes behave.
//!
//! Cards covered (9), so this is greppable by name as well as by rule:
//!
//! - Bloodline Keeper
//! - Civilized Scholar
//! - Cloistered Youth
//! - Delver of Secrets
//! - Garruk Relentless
//! - Ludevic's Test Subject
//! - Mikaeus, the Lunarch
//! - Screeching Bat
//! - Thraben Sentry

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use mtg_engine::cards::helpers;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::Step;
use mtg_engine::actions::Action;
use mtg_engine::actions::ResolvedChoice;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;

// -------------------------------------------------------------------------
// Delver of Secrets
// -------------------------------------------------------------------------

#[test]
fn delver_transforms_when_player_reveals_instant() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    // Put Delver on the battlefield.
    let delver = named_permanent(&mut state, &reg, "Delver of Secrets", P0);
    assert_eq!(state.get_object(delver).unwrap().power, Some(1));

    // Put a Lightning Bolt (instant) on top of library.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Library, &reg);
    state.players[0].library_order.insert(0, bolt);

    // Trigger upkeep — should present a YesNo choice.
    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &[], &reg);

    // Should NOT be transformed yet — awaiting player choice.
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert!(state.awaiting_action.is_some(), "Should be awaiting reveal choice");

    // Player chooses to reveal.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Now should be transformed.
    assert!(state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Insectile Aberration");
    // Dynamic P/T should be 3/2.
    assert_eq!(
        (state.effective_power(delver, &reg), state.effective_toughness(delver, &reg)),
        (Some(3), Some(2)),
        "the back face's printed size (CR 712.8)");

    // The card should still be on top of the library (per ruling).
    assert_eq!(state.players[0].library_order.first().copied(), Some(bolt));
}

#[test]
fn delver_does_not_transform_when_player_declines_reveal() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let delver = named_permanent(&mut state, &reg, "Delver of Secrets", P0);

    // Put a Lightning Bolt (instant) on top of library.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Library, &reg);
    state.players[0].library_order.insert(0, bolt);

    // Trigger upkeep — should present a YesNo choice.
    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &[], &reg);

    assert!(state.awaiting_action.is_some(), "Should be awaiting reveal choice");

    // Player declines to reveal.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) },
        &reg,
    );

    // Should NOT be transformed.
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Delver of Secrets");

    // The card should still be on top of the library.
    assert_eq!(state.players[0].library_order.first().copied(), Some(bolt));
}

#[test]
fn delver_does_not_transform_when_top_card_is_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let delver = named_permanent(&mut state, &reg, "Delver of Secrets", P0);

    // Put a creature on top of library.
    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(creature, Zone::Library, &reg);
    state.players[0].library_order.insert(0, creature);

    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &[], &reg);

    // Per oracle ruling: "You may reveal the card even if it's not an instant or sorcery."
    // A choice should be presented. If the player reveals, Delver does NOT transform
    // (since it's a creature, not an instant or sorcery).
    assert!(state.awaiting_action.is_some(), "Should present 'you may reveal' choice");

    // Player chooses to reveal.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Should NOT transform (top card is a creature).
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Delver of Secrets");
}

// -------------------------------------------------------------------------
// Cloistered Youth
// -------------------------------------------------------------------------

#[test]
fn cloistered_youth_presents_transform_choice_at_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let youth = named_permanent(&mut state, &reg, "Cloistered Youth", P0);

    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, youth, &[], &reg);

    // Should NOT be transformed yet — awaiting player choice.
    assert!(!state.get_object(youth).unwrap().is_transformed);
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses yes to transform.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Now should be transformed.
    assert!(state.get_object(youth).unwrap().is_transformed);
    assert_eq!(state.get_object(youth).unwrap().name, "Unholy Fiend");
    assert_eq!(
        (state.effective_power(youth, &reg), state.effective_toughness(youth, &reg)),
        (Some(3), Some(3)),
        "the back face's printed size (CR 712.8)");
}

#[test]
fn cloistered_youth_can_decline_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let youth = named_permanent(&mut state, &reg, "Cloistered Youth", P0);

    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, youth, &[], &reg);

    // Should be awaiting player choice.
    assert!(state.awaiting_action.is_some());

    // Player declines to transform.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) },
        &reg,
    );

    // Should NOT be transformed.
    assert!(!state.get_object(youth).unwrap().is_transformed);
    assert_eq!(state.get_object(youth).unwrap().name, "Cloistered Youth");
}

#[test]
fn unholy_fiend_drains_life_at_end_step() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let youth = named_permanent(&mut state, &reg, "Cloistered Youth", P0);
    // Pre-transform.
    state.get_object_mut(youth).unwrap().is_transformed = true;
    state.get_object_mut(youth).unwrap().name = "Unholy Fiend".into();

    let life_before = state.players[0].life;
    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_end_step(&mut state, youth, &[], &reg);

    assert_eq!(state.players[0].life, life_before - 1);
}

// -------------------------------------------------------------------------
// Screeching Bat
// -------------------------------------------------------------------------

#[test]
fn screeching_bat_transforms_at_upkeep_when_player_pays() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);
    assert!(!state.get_object(bat).unwrap().is_transformed);

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);

    // Should NOT be transformed yet — awaiting player choice.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert!(state.awaiting_action.is_some(), "Should be awaiting pay choice");

    // Player chooses to pay.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Now should be transformed.
    assert!(state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Stalking Vampire");
    assert_eq!(
        (state.effective_power(bat, &reg), state.effective_toughness(bat, &reg)),
        (Some(5), Some(5)),
        "the back face's printed size (CR 712.8)");

    // Mana should have been spent.
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);
}

#[test]
fn screeching_bat_does_not_transform_when_player_declines() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);
    assert!(!state.get_object(bat).unwrap().is_transformed);

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);

    assert!(state.awaiting_action.is_some(), "Should be awaiting pay choice");

    // Player declines to pay.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) },
        &reg,
    );

    // Should NOT be transformed.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Screeching Bat");

    // Mana should NOT have been spent.
    assert_eq!(state.get_player(P0).mana_pool.total(), 4);
}

#[test]
fn screeching_bat_no_choice_without_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);

    // No mana in pool — choice should not be presented.
    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);

    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert!(state.awaiting_action.is_none(), "No choice should be presented without mana");
}

#[test]
fn stalking_vampire_transforms_back_when_player_pays() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);

    // Transform to Stalking Vampire first.
    if let Some(obj) = state.get_object_mut(bat) {
        obj.is_transformed = true;
        obj.name = "Stalking Vampire".into();
    }

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);

    // Should be awaiting choice.
    assert!(state.awaiting_action.is_some(), "Should be awaiting pay choice");

    // Player chooses to pay.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Should transform back to Screeching Bat.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Screeching Bat");
}

#[test]
fn stalking_vampire_does_not_have_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);

    // (No hand-seeding of obj.keywords: printed keywords live on the card's
    // active face, so `has_keyword` reads them straight from the registry.)

    // Verify front face has Flying.
    assert!(state.has_keyword(bat, Keyword::Flying, &reg));

    // Add mana and transform.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Now Stalking Vampire — should NOT have Flying.
    assert!(state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Stalking Vampire");
    assert!(!state.has_keyword(bat, Keyword::Flying, &reg),
        "Stalking Vampire should not have Flying");
}

#[test]
fn screeching_bat_regains_flying_on_transform_back() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);

    // Start already on the back face. Flipping `is_transformed` is the whole
    // of it — the characteristics accessors read the active face.
    if let Some(obj) = state.get_object_mut(bat) {
        obj.is_transformed = true;
        obj.name = "Stalking Vampire".into();
    }

    // Add mana and transform back.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Should be back to Screeching Bat with Flying restored.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Screeching Bat");
    assert!(state.has_keyword(bat, Keyword::Flying, &reg),
        "Screeching Bat should have Flying after transforming back");
}

#[test]
fn screeching_bat_transform_updates_subtypes() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_permanent(&mut state, &reg, "Screeching Bat", P0);

    // Front face is a Bat, per the registry — nothing to seed.
    assert!(state.has_subtype(bat, "Bat", &reg));

    // Add mana and transform.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Stalking Vampire should have "Vampire" subtype, not "Bat".
    assert!(state.has_subtype(bat, "Vampire", &reg),
        "Stalking Vampire should have Vampire subtype");
    assert!(!state.has_subtype(bat, "Bat", &reg),
        "Stalking Vampire should not have Bat subtype");
}

// -------------------------------------------------------------------------
// Ludevic's Test Subject
// -------------------------------------------------------------------------

#[test]
fn ludevics_test_subject_transforms_at_five_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let subject = named_permanent(&mut state, &reg, "Ludevic's Test Subject", P0);
    assert_eq!(state.get_object(subject).unwrap().power, Some(0));


    // Activate 4 times — should not transform yet.
    for _ in 0..4 {
        activate_via_hooks(&mut state, &reg, subject, 0, &[]);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    }
    assert!(!state.get_object(subject).unwrap().is_transformed);

    // 5th activation — should transform.
    activate_via_hooks(&mut state, &reg, subject, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    let obj = state.get_object(subject).unwrap();
    assert!(obj.is_transformed);
    assert_eq!(obj.name, "Ludevic's Abomination");
    assert_eq!(
        (state.effective_power(subject, &reg), state.effective_toughness(subject, &reg)),
        (Some(13), Some(13)),
        "the back face's printed size (CR 712.8)");
    // The back face's keywords and subtypes come from the active face.
    assert!(state.has_keyword(subject, Keyword::Trample, &reg), "back face should have Trample");
    assert!(!state.has_keyword(subject, Keyword::Defender, &reg), "back face should not have Defender");
    assert!(state.has_subtype(subject, "Lizard", &reg));
    assert!(state.has_subtype(subject, "Horror", &reg));
}

// -------------------------------------------------------------------------
// Thraben Sentry
// -------------------------------------------------------------------------

#[test]
fn thraben_sentry_transforms_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sentry = named_permanent(&mut state, &reg, "Thraben Sentry", P0);
    let other = ready_creature(&mut state, P0, 1, 1);

    assert!(!state.get_object(sentry).unwrap().is_transformed);

    // Simulate another creature dying — presents "you may transform" choice.
    let behavior = reg.get(state.get_object(sentry).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, sentry, other, P0, &[], 1, false, &[], &reg);

    // Oracle: "you may transform" — choice should be presented.
    assert!(state.awaiting_action.is_some(), "Should present 'you may transform' choice");

    // Player chooses yes.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    assert!(state.get_object(sentry).unwrap().is_transformed);
    assert_eq!(state.get_object(sentry).unwrap().name, "Thraben Militia");
    assert_eq!(
        (state.effective_power(sentry, &reg), state.effective_toughness(sentry, &reg)),
        (Some(5), Some(4)),
        "Thraben Militia is a 5/4 — its back face's printed size (CR 712.8)");
}

#[test]
fn thraben_sentry_does_not_transform_when_opponent_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sentry = named_permanent(&mut state, &reg, "Thraben Sentry", P0);
    let opp_creature = ready_creature(&mut state, P1, 1, 1);

    let behavior = reg.get(state.get_object(sentry).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, sentry, opp_creature, P1, &[], 1, false, &[], &reg);

    // Should NOT transform.
    assert!(!state.get_object(sentry).unwrap().is_transformed);
}

// -------------------------------------------------------------------------
// Bloodline Keeper
// -------------------------------------------------------------------------

#[test]
fn bloodline_keeper_creates_vampire_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let keeper = named_permanent(&mut state, &reg, "Bloodline Keeper", P0);

    activate_via_hooks(&mut state, &reg, keeper, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have a Vampire token.
    assert_eq!(count_tokens_named(&state, "Vampire Token"), 1);
    let token = find_token_named(&state, "Vampire Token").unwrap();
    let obj = state.get_object(token).unwrap();
    assert_eq!(obj.power, Some(2));
    assert_eq!(obj.toughness, Some(2));
}

// -------------------------------------------------------------------------
// Mikaeus, the Lunarch
// -------------------------------------------------------------------------

#[test]
fn mikaeus_enters_with_x_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create Mikaeus with x_value = 3.
    let card_id = reg.get_id_by_name("Mikaeus, the Lunarch").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, Some(0), Some(0));
    state.get_object_mut(id).unwrap().name = "Mikaeus, the Lunarch".into();
    state.get_object_mut(id).unwrap().x_value = Some(3);

    let behavior = reg.get(card_id).unwrap();
    behavior.on_resolve(&mut state, id, &[], &reg);

    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_counter_count(id, CounterType::PlusOnePlusOne), 3);
}

#[test]
fn mikaeus_distributes_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mikaeus = named_permanent(&mut state, &reg, "Mikaeus, the Lunarch", P0);
    // Give Mikaeus 2 +1/+1 counters.
    state.add_counters(mikaeus, CounterType::PlusOnePlusOne, 2);

    let other1 = ready_creature(&mut state, P0, 2, 2);
    let other2 = ready_creature(&mut state, P0, 1, 1);

    // Use ability 1: remove a counter, give +1/+1 to each other creature.
    activate_via_hooks(&mut state, &reg, mikaeus, 1, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Mikaeus should have lost a counter.
    assert_eq!(state.get_counter_count(mikaeus, CounterType::PlusOnePlusOne), 1);
    // Other creatures should each have a counter.
    assert_eq!(state.get_counter_count(other1, CounterType::PlusOnePlusOne), 1);
    assert_eq!(state.get_counter_count(other2, CounterType::PlusOnePlusOne), 1);
}

// -------------------------------------------------------------------------
// Garruk Relentless
// -------------------------------------------------------------------------

#[test]
fn garruk_creates_wolf_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 3);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 1, &[], &reg);

    assert_eq!(count_tokens_named(&state, "Wolf Token"), 1);
    let wolf = find_token_named(&state, "Wolf Token").unwrap();
    assert_eq!(state.get_object(wolf).unwrap().power, Some(2));
}

#[test]
fn garruk_transforms_at_two_or_fewer_loyalty() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 2);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // Use the wolf token ability (costs 0 loyalty).
    behavior.on_loyalty_ability(&mut state, garruk, 1, &[], &reg);

    // Transform is now a state-triggered ability: SBA detects the condition and
    // queues a trigger, which then goes on the stack and resolves.
    check_state_based_actions(&mut state, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Should have transformed.
    assert!(state.get_object(garruk).unwrap().is_transformed);
    assert_eq!(state.get_object(garruk).unwrap().name, "Garruk, the Veil-Cursed");
}

#[test]
fn garruk_back_face_creates_deathtouch_wolf() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 2);
    helpers::apply_transform(&mut state, garruk, &reg);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // +1: Create a 1/1 black Wolf with deathtouch (ability_index 10).
    behavior.on_loyalty_ability(&mut state, garruk, 10, &[], &reg);

    assert_eq!(count_tokens_named(&state, "Wolf Token"), 1, "Should create a Wolf token");
    let wolf = find_token_named(&state, "Wolf Token").unwrap();
    let obj = state.get_object(wolf).unwrap();
    assert_eq!(obj.power, Some(1), "Wolf should be 1/1");
    assert_eq!(obj.toughness, Some(1), "Wolf should be 1/1");
    assert!(obj.keywords.contains(&Keyword::Deathtouch), "Wolf should have deathtouch");
}

#[test]
fn garruk_back_face_sacrifice_to_tutor() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 3);
    helpers::apply_transform(&mut state, garruk, &reg);

    // Put a creature on the battlefield to sacrifice.
    let sac_target = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(sac_target).unwrap().card_types = vec![CardType::Creature];

    // Put a creature card in the library.
    let lib_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(lib_creature, Zone::Library, &reg);
    state.get_player_mut(P0).library_order.push(lib_creature);
    if let Some(obj) = state.get_object_mut(lib_creature) {
        obj.card_types = vec![CardType::Creature];
    }

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // -1: Sacrifice a creature, search for a creature card (ability_index 11).
    behavior.on_loyalty_ability(&mut state, garruk, 11, &[], &reg);

    // Sac target should be in graveyard.
    assert_eq!(state.get_object(sac_target).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Library creature should now be in hand.
    assert_eq!(state.get_object(lib_creature).unwrap().zone, Zone::Hand,
        "Tutored creature should be in hand");
}

#[test]
fn garruk_back_face_tutor_presents_sacrifice_choice() {
    // With multiple creatures, the -1 ability should present a sacrifice choice.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 4);
    helpers::apply_transform(&mut state, garruk, &reg);

    // Two creatures to choose from.
    let creature1 = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature1).unwrap().card_types = vec![CardType::Creature];
    let creature2 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature2).unwrap().card_types = vec![CardType::Creature];

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 11, &[], &reg);

    // Should be awaiting a sacrifice choice (not auto-selected).
    assert!(state.awaiting_action.is_some(),
        "Should present sacrifice choice when multiple creatures available");

    // Resolve with the specific creature we want to sacrifice.
    let new_state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(mtg_engine::actions::Target::Object(creature1))),
        },
        &reg,
    );

    // creature1 should be sacrificed (in graveyard).
    assert_eq!(new_state.get_object(creature1).unwrap().zone, Zone::Graveyard,
        "Chosen creature should be sacrificed");
    // creature2 should still be on the battlefield.
    assert_eq!(new_state.get_object(creature2).unwrap().zone, Zone::Battlefield,
        "Non-chosen creature should remain on battlefield");
}

/// Garruk, the Veil-Cursed's -1: "Search your library for a creature card, reveal
/// it, put it into your hand, then shuffle." The shuffle is the part that is easy
/// to leave out, so assert it: over repeated runs the remaining library must come
/// back in more than one order. A single run cannot tell a shuffle from a no-op.
#[test]
fn garruk_back_face_tutor_shuffles_library() {
    let reg = registry();

    let run = || {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
        set_loyalty(&mut state, garruk, 4);
        helpers::apply_transform(&mut state, garruk, &reg);

        let sac_target = ready_creature(&mut state, P0, 1, 1);
        state.get_object_mut(sac_target).unwrap().card_types = vec![CardType::Creature];

        let mut lib_ids = Vec::new();
        for name in &["Grizzly Bears", "Doom Blade", "Giant Growth", "Divination", "Lightning Bolt"] {
            let id = spell_in_hand(&mut state, &reg, name, P0);
            state.move_object(id, Zone::Library, &reg);
            state.get_player_mut(P0).library_order.push(id);
            if *name == "Grizzly Bears" {
                state.get_object_mut(id).unwrap().card_types = vec![CardType::Creature];
            }
            lib_ids.push(id);
        }
        let before = state.get_player(P0).library_order.clone();

        let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
        behavior.on_loyalty_ability(&mut state, garruk, 11, &[], &reg);

        let after = state.get_player(P0).library_order.clone();
        assert!(!after.contains(&lib_ids[0]),
            "the tutored creature is in hand, not still in the library");
        assert_eq!(after.len(), before.len() - 1,
            "exactly one card left the library");
        assert_eq!(state.get_object(lib_ids[0]).unwrap().zone, Zone::Hand,
            "the tutored creature ends up in hand");
        after
    };

    let first = run();
    // Four cards remain, so an unshuffled library repeats the same order every
    // time. Twenty runs that all agree would be a 1-in-24^19 coincidence.
    let shuffled = (0..20).any(|_| run() != first);
    assert!(shuffled, "the library came back in the same order 20 times — it was never shuffled");
}

#[test]
fn garruk_back_face_overrun() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 4);
    helpers::apply_transform(&mut state, garruk, &reg);

    // Put 2 creature cards in graveyard.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
        state.move_object(c, Zone::Graveyard, &reg);
    }

    // Put a creature on the battlefield.
    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().card_types = vec![CardType::Creature];

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // -3: Creatures get +X/+X and trample (ability_index 12).
    behavior.on_loyalty_ability(&mut state, garruk, 12, &[], &reg);

    // X should be 2 (2 creature cards in graveyard).
    // Creature should have +2/+2 until end of turn.
    let has_buff = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::ModifyPT { target, power_mod, toughness_mod } if *target == creature && *power_mod == 2 && *toughness_mod == 2));
    assert!(has_buff, "Creature should have +2/+2 until end of turn");

    // Should have trample.
    let has_trample = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantKeyword { target, keyword } if *target == creature && *keyword == Keyword::Trample));
    assert!(has_trample, "Creature should have trample until end of turn");
}

#[test]
fn garruk_back_face_loyalty_abilities_shown_when_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 3);
    helpers::apply_transform(&mut state, garruk, &reg);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    let abilities = behavior.loyalty_abilities(&state, garruk);

    // Back face should have 3 abilities with indices 10, 11, 12.
    assert_eq!(abilities.len(), 3, "Back face should have 3 loyalty abilities");
    assert_eq!(abilities[0].ability_index, 10);
    assert_eq!(abilities[0].loyalty_change, 1); // +1
    assert_eq!(abilities[1].ability_index, 11);
    assert_eq!(abilities[1].loyalty_change, -1); // -1
    assert_eq!(abilities[2].ability_index, 12);
    assert_eq!(abilities[2].loyalty_change, -3); // -3
}

// -------------------------------------------------------------------------
// Civilized Scholar
// -------------------------------------------------------------------------

#[test]
fn civilized_scholar_discard_creature_transforms() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scholar = named_permanent(&mut state, &reg, "Civilized Scholar", P0);

    // Put a card in the library (will be drawn).
    let lib_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(lib_card, Zone::Library, &reg);
    state.players[0].library_order = vec![lib_card];

    // Put a non-creature in hand (so we have a choice after drawing).
    let _hand_spell = spell_in_hand(&mut state, &reg, "Doom Blade", P0);

    // Activate the ability — draws a card, then prompts for discard.
    let new_state = activate(&state, &reg, scholar, 0, vec![]);

    // Should be awaiting a discard choice now.
    assert!(new_state.awaiting_action.is_some(),
        "Should be awaiting discard choice");

    // Choose to discard the creature (Grizzly Bears we drew).
    let new_state = engine::submit_action(
        &new_state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(lib_card) },
        &reg,
    );

    // Should have transformed (discarded a creature).
    assert!(new_state.get_object(scholar).unwrap().is_transformed,
        "Should transform after discarding a creature");
    assert_eq!(new_state.get_object(scholar).unwrap().name, "Homicidal Brute");
    assert!(!new_state.get_object(scholar).unwrap().tapped,
        "Should be untapped after transform");
}

#[test]
fn civilized_scholar_discard_noncreature_no_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scholar = named_permanent(&mut state, &reg, "Civilized Scholar", P0);

    // Put a card in the library (will be drawn).
    let lib_card = spell_in_hand(&mut state, &reg, "Doom Blade", P0);
    state.move_object(lib_card, Zone::Library, &reg);
    state.players[0].library_order = vec![lib_card];

    // Put a creature in hand.
    let hand_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    // Activate the ability.
    let new_state = activate(&state, &reg, scholar, 0, vec![]);

    // Should be awaiting a discard choice.
    assert!(new_state.awaiting_action.is_some());

    // Choose to discard the non-creature (Doom Blade we drew), keeping the creature.
    let new_state = engine::submit_action(
        &new_state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(lib_card) },
        &reg,
    );

    // Should NOT transform (discarded a non-creature).
    assert!(!new_state.get_object(scholar).unwrap().is_transformed,
        "Should NOT transform after discarding a non-creature");
    assert_eq!(new_state.get_object(scholar).unwrap().name, "Civilized Scholar");
    // Should still be tapped (no untap since no transform).
    assert!(new_state.get_object(scholar).unwrap().tapped,
        "Should remain tapped when no transform");

    // The creature should still be in hand.
    assert_eq!(new_state.get_object(hand_creature).unwrap().zone, Zone::Hand,
        "Player chose to keep the creature in hand");
}

// -------------------------------------------------------------------------
// Garruk Relentless — the rest
// -------------------------------------------------------------------------

// CR 702.16e: protection prevents all damage from sources with the matching quality.
// Garruk's ability 0 deals 3 damage inline (damage_marked += 3), skipping has_protection_from.
#[test]
fn garruk_damage_respects_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 3);
    state.get_object_mut(garruk).unwrap().subtypes.push("Green".into());

    let creature = ready_creature(&mut state, P1, 2, 4);
    state.get_object_mut(creature).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Green".into(),
            scope: EffectScope::OnSelf,
        },
    ]);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 0, &[Target::Object(creature)], &reg);

    assert_eq!(
        state.get_object(creature).unwrap().damage_marked, 0,
        "CR 702.16e: creature with protection from green takes 0 damage from Garruk"
    );
}

// CR 702.15b: a source with lifelink causes its controller to gain life
// equal to the damage dealt. Garruk's ability 0 has the creature deal power
// as damage to Garruk by directly decrementing loyalty counters, bypassing
// the lifelink check in apply_pending_effect.
#[test]
fn garruk_fight_creature_lifelink_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 3);

    let creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Lifelink);

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 0, &[Target::Object(creature)], &reg);

    assert_eq!(
        state.players[1].life, 22,
        "CR 702.15b: opponent gains 2 life from lifelink creature dealing 2 damage to Garruk"
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Garruk Relentless doesn't set `is_legendary` in `on_resolve`,
/// so the legend rule may not apply to it.
#[test]
fn bug_garruk_relentless_not_legendary_on_battlefield() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cast Garruk
    let garruk = castable_spell(&mut state, &registry, "Garruk Relentless", P0);
    state = cast_and_resolve(&state, &registry, garruk, vec![]);

    // Check if Garruk has the Legendary supertype recognized
    let card_id = state.get_object(garruk).unwrap().card_id;
    let data = registry.card_data(card_id).unwrap();
    let is_legendary_in_data = data.supertypes.contains(&Supertype::Legendary);

    // Also check the object-level flag
    let obj_legendary = state.get_object(garruk).is_some_and(|o| o.is_legendary);

    // BUG: is_legendary not set on the object
    assert!(is_legendary_in_data, "Garruk should be Legendary in card data");
    assert!(obj_legendary,
        "Garruk should have is_legendary set on the object for legend rule enforcement");
}

// -------------------------------------------------------------------------
// Civilized Scholar — the rest
// -------------------------------------------------------------------------

const P0: PlayerId = PlayerId(0);

#[test]
fn front_face_civilized_scholar_has_no_end_step_trigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    assert!(!state.get_object(scholar).unwrap().is_transformed,
        "setup sanity: scholar should be on front face");

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger { source: TriggerSource { .. }, event: TriggerEvent::EndStep })
    )).count();
    assert_eq!(end_step_triggers, 0,
        "Front-face Civilized Scholar has no EndStep trigger per oracle");
}

#[test]
fn back_face_homicidal_brute_has_end_step_trigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);

    // Transform to Homicidal Brute (back face) via the shared helper.
    helpers::apply_transform(&mut state, scholar, &registry);
    assert!(state.get_object(scholar).unwrap().is_transformed);
    assert_eq!(state.get_object(scholar).unwrap().name, "Homicidal Brute");

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger { source: TriggerSource { .. }, event: TriggerEvent::EndStep })
    )).count();
    assert_eq!(end_step_triggers, 1,
        "Back-face Homicidal Brute should fire its end-step transform-back trigger");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Ruling: "If Civilized Scholar attacks, and later in the turn (but before the
/// beginning of your end step), it transforms, Homicidal Brute's last ability
/// won't trigger. This is because the creature attacked that turn, even if it
/// had its other face up at the time."
///
/// CR 712.8: transforming does not make a new object, so the attack follows the
/// permanent across the flip. Attacks are declared through the engine here
/// rather than by poking a marker, because the point of the test is that the
/// engine records the attack at all.
#[test]
fn an_attack_before_transforming_still_counts_for_the_back_face() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.active_player = P0;

    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    state.get_object_mut(scholar).unwrap().summoning_sick = false;

    // It attacks with its front face up.
    mtg_engine::combat::declare_attackers(&mut state, &[(scholar, P1)], &registry);
    assert!(state.attacked_this_turn(scholar),
        "declaring it as an attacker is what makes it have attacked");

    // Then transforms, later in the same turn.
    helpers::apply_transform(&mut state, scholar, &registry);
    assert_eq!(state.get_object(scholar).unwrap().name, "Homicidal Brute");
    assert!(state.attacked_this_turn(scholar),
        "CR 712.8: the flip does not make a new object, so the attack still counts");

    // End step: the Brute's ability does not trigger, so it stays flipped.
    state.step = Step::EndStep;
    let behavior = registry.get(state.get_object(scholar).unwrap().card_id).unwrap();
    behavior.on_end_step(&mut state, scholar, &[], &registry);
    assert!(state.get_object(scholar).unwrap().is_transformed,
        "it attacked this turn, so it does not transform back");
}

/// Civilized Scholar has exactly one ability: the activated draw-and-discard.
/// It used to declare an `Attacks` triggered ability on each face whose only
/// job was to record that it had attacked — and triggers go on the stack, so
/// attacking put a visible, respondable ability on the stack that the card does
/// not have. The attack is a fact the engine records now.
#[test]
fn civilized_scholar_attacking_puts_nothing_on_the_stack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.active_player = P0;

    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    state.get_object_mut(scholar).unwrap().summoning_sick = false;

    mtg_engine::combat::declare_attackers(&mut state, &[(scholar, P1)], &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    assert!(state.stack.is_empty(),
        "Civilized Scholar has no attack trigger; nothing belongs on the stack");
    assert!(state.attacked_this_turn(scholar),
        "and the attack is recorded regardless");
}

/// A creature that leaves the battlefield and comes back is a new object
/// (CR 400.7) — it has not attacked, even in the same turn.
#[test]
fn returning_to_the_battlefield_clears_the_attack(){
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.active_player = P0;

    let scholar = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    state.get_object_mut(scholar).unwrap().summoning_sick = false;
    mtg_engine::combat::declare_attackers(&mut state, &[(scholar, P1)], &registry);
    assert!(state.attacked_this_turn(scholar));

    state.move_object(scholar, Zone::Graveyard, &registry);
    state.move_object(scholar, Zone::Battlefield, &registry);
    assert!(!state.attacked_this_turn(scholar),
        "what came back has not attacked");
}

/// CR 603.4: "At the beginning of your end step, **if** this creature didn't
/// attack this turn, tap this creature, then transform it" is an intervening-if
/// clause. The condition is checked when the ability would trigger, not only
/// when it resolves — if it is false the ability never goes on the stack, so
/// the phantom entry and the priority window it opens must not exist.
///
/// `back_face_homicidal_brute_has_end_step_trigger` above covers the trigger
/// firing when the Brute did not attack. This is the other half.
#[test]
fn homicidal_brute_that_attacked_this_turn_puts_no_trigger_on_the_stack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndStep, P0);
    state.active_player = P0;

    let brute = named_permanent(&mut state, &registry, "Civilized Scholar", P0);
    helpers::apply_transform(&mut state, brute, &registry);
    let turn = state.turn_number;
    state.get_object_mut(brute).unwrap().attacked_on_turn = Some(turn);

    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let end_step_triggers = state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(PendingTrigger { event: TriggerEvent::EndStep, .. })
    )).count();
    assert_eq!(end_step_triggers, 0,
        "it attacked this turn, so the intervening-if is false and the ability \
         never triggers (CR 603.4)");
}

/// Scryfall ruling (2011-09-22): "You can't activate a loyalty ability of
/// Garruk Relentless and later that turn after he transforms activate a
/// loyalty ability of Garruk, the Veil-Cursed."
///
/// CR 606.3 limits a *permanent* to one loyalty ability a turn, and CR 711.5
/// says transforming does not make a new object — so the front face's
/// activation still counts against the back face. The two faces number their
/// abilities differently (0/1 and 10/11/12), so a per-index limit would let
/// both through; the engine tracks a per-permanent sentinel instead.
#[test]
fn garruk_cannot_activate_a_loyalty_ability_on_each_face_in_one_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 5);

    let loyalty_actions = |s: &mtg_engine::state::GameState| {
        mtg_engine::engine::legal_actions(s, &reg).actions.iter()
            .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { object_id, .. } if *object_id == garruk))
            .count()
    };
    assert!(loyalty_actions(&state) > 0, "setup: the front face offers loyalty abilities");

    // Use the front face's "create a 2/2 Wolf" (index 1, no target).
    let state = mtg_engine::engine::submit_action(&state,
        &Action::ActivateLoyaltyAbility { object_id: garruk, ability_index: 1, targets: vec![] }, &reg);
    assert_eq!(loyalty_actions(&state), 0, "one loyalty ability per turn (CR 606.3)");

    // Now transform him. The activation must survive the flip.
    let mut state = state;
    helpers::apply_transform(&mut state, garruk, &reg);
    assert!(state.get_object(garruk).unwrap().is_transformed, "setup: transformed");

    assert_eq!(loyalty_actions(&state), 0,
        "the back face's abilities are numbered differently, but it is the same \
         permanent — it already used a loyalty ability this turn");
}

/// Every double-faced creature in the set, flipped, is its back face's printed
/// size (CR 712.8).
///
/// Derived from the registry rather than a hand-written list, and read through
/// `effective_power`/`effective_toughness` rather than any card hook: nineteen
/// DFCs each carried a `dynamic_pt` that only restated their own
/// `back_face_data`, and every test that covered a flip asserted the *hook*,
/// so the two could have disagreed without anything noticing.
#[test]
fn every_transformed_dfc_is_its_back_faces_printed_size() {
    let reg = registry();
    let mut checked = 0;
    for name in reg.all_names() {
        let Some(card_id) = reg.get_id_by_name(name) else { continue };
        let Some(behavior) = reg.get(card_id) else { continue };
        let Some(back) = behavior.back_face_data() else { continue };
        let (Some(bp), Some(bt)) = (back.power, back.toughness) else { continue };
        let front = behavior.card_data();
        let (Some(fp), Some(ft)) = (front.power, front.toughness) else { continue };

        let mut state = game_at_step(Step::PrecombatMain, P0);
        let id = named_permanent(&mut state, &reg, name, P0);
        assert_eq!(
            (state.effective_power(id, &reg), state.effective_toughness(id, &reg)),
            (Some(fp), Some(ft)),
            "{name}: front face is printed {fp}/{ft}");

        state.get_object_mut(id).unwrap().is_transformed = true;
        assert_eq!(
            (state.effective_power(id, &reg), state.effective_toughness(id, &reg)),
            (Some(bp), Some(bt)),
            "{name} transformed is {}, printed {bp}/{bt}", back.name);
        checked += 1;
    }
    assert!(checked >= 15,
        "only {checked} double-faced creatures checked — the sweep stopped covering the set");
}

/// A transformed Garruk is named Garruk, the Veil-Cursed, and his oracle text is
/// the back face's. `state.name_of` is the authoritative accessor — `obj.name`
/// is a display cache, per the doc comment on `name_of` itself — so a DFC that
/// does not declare its back face reports the front face's name and rules text
/// forever, whatever a hand-written `obj.name` says. That reaches the legend
/// rule (CR 704.5j) and anything matching on names.
#[test]
fn a_transformed_garruk_reports_his_back_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    assert_eq!(state.name_of(garruk, &reg), "Garruk Relentless");

    helpers::apply_transform(&mut state, garruk, &reg);

    assert_eq!(state.name_of(garruk, &reg), "Garruk, the Veil-Cursed",
        "the active face names him");
    let text = state.face_data(garruk, &reg).unwrap().oracle_text;
    assert!(text.contains("deathtouch"),
        "the active face carries the back face's rules text, got: {text}");
    assert!(!text.contains("two or fewer loyalty counters"),
        "and not the front face's, got: {text}");
}

/// CR 113.7a: "At the beginning of your end step, you lose 1 life" names
/// nothing about the Unholy Fiend, so killing it in response to its own trigger
/// does not save the life.
///
/// The handler used to require the source on the battlefield *and* still
/// transformed — and leaving the battlefield clears `is_transformed`, so a dead
/// Fiend failed both checks and the life loss silently vanished.
#[test]
fn unholy_fiends_life_loss_happens_even_if_it_dies_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let youth = named_permanent(&mut state, &reg, "Cloistered Youth", P0);
    helpers::apply_transform(&mut state, youth, &reg);
    assert_eq!(state.name_of(youth, &reg), "Unholy Fiend");
    let before = state.get_player(P0).life;

    // The trigger goes on the stack...
    state.events.push(GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);
    assert!(!state.stack.is_empty(), "the end-step trigger is on the stack");

    // ...and the Fiend is killed in response.
    state.move_object(youth, Zone::Graveyard, &reg);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, before - 1,
        "the life is lost even though the source is gone");
}

/// "look at the top card of your library. You may reveal that card." — with an
/// empty library there is no card to look at and nothing to reveal, so there is
/// no choice to offer (CR 608.2: the ability does as much as it can, which here
/// is nothing). It used to prompt "reveal nothing from the top of your
/// library?", a decision with nothing behind it.
#[test]
fn delver_offers_no_reveal_when_the_library_is_empty() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let delver = named_permanent(&mut state, &reg, "Delver of Secrets", P0);
    assert!(state.get_player(P0).library_order.is_empty(), "test precondition");

    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &[], &reg);

    assert!(state.awaiting_action.is_none(),
        "no card to look at, so no reveal choice");
    assert!(!state.get_object(delver).unwrap().is_transformed);
}

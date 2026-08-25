//! Cards that change the rules of the game rather than affecting objects:
//! alternative and reduced costs, a replacement for losing the game, granted
//! flashback, doubled token creation, a banned card name.
//!
//! Cards covered (10), so this is greppable by name as well as by rule:
//!
//! - Devil's Play
//! - Heartless Summoning
//! - Kessig Wolf
//! - Kessig Wolf Run
//! - Laboratory Maniac
//! - Nevermore
//! - Parallel Lives
//! - Past in Flames
//! - Rooftop Storm
//! - Snapcaster Mage

mod common;

use common::*;
use mtg_engine::engine;
use mtg_engine::actions::{Action, Target};
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
// ── Laboratory Maniac ──────────────────────────────────────────

/// Laboratory Maniac: player wins when drawing from empty library.
#[test]
fn laboratory_maniac_wins_on_empty_library_draw() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Lab Maniac on the battlefield for P0.
    let _lab_maniac = named_creature(&mut state, &reg, "Laboratory Maniac", P0);

    // Empty P0's library.
    state.players[0].library_order.clear();

    // Attempt to draw a card.
    let _ = engine::draw_cards(&mut state, P0, 1, &reg);

    // P0 should win the game (opponent loses).
    assert!(state.result.is_some(), "Game should be over");
    assert!(state.players[1].lost, "Opponent should have lost");
    assert!(!state.players[0].lost, "Lab Maniac player should not have lost");
}

/// Without Laboratory Maniac, drawing from empty library loses the game.
#[test]
fn no_lab_maniac_loses_on_empty_draw() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Empty P0's library.
    state.players[0].library_order.clear();

    // Attempt to draw a card.
    let _ = engine::draw_cards(&mut state, P0, 1, &reg);

    // P0 should be flagged for SBA loss.
    assert!(state.players[0].has_drawn_from_empty);

    // SBA should kill P0.
    check_state_based_actions(&mut state, &reg);
    assert!(state.players[0].lost, "Player without Lab Maniac should lose");
}

/// Lab Maniac only helps its controller.
#[test]
fn laboratory_maniac_only_helps_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Lab Maniac on P0's battlefield, but P1 draws from empty.
    let _lab_maniac = named_creature(&mut state, &reg, "Laboratory Maniac", P0);

    state.players[1].library_order.clear();
    let _ = engine::draw_cards(&mut state, P1, 1, &reg);

    // P1 should still lose (Lab Maniac only helps P0).
    assert!(state.players[1].has_drawn_from_empty);
    check_state_based_actions(&mut state, &reg);
    assert!(state.players[1].lost);
}

// ── Parallel Lives ──────────────────────────────────────────

/// Parallel Lives doubles token creation.
#[test]
fn parallel_lives_doubles_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Parallel Lives on the battlefield.
    let _pl = named_creature(&mut state, &reg, "Parallel Lives", P0);

    // Create one token.
    state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
        &reg,
    );

    // Should have 2 tokens (1 + 1 extra from Parallel Lives).
    assert_eq!(count_tokens_named_by(&state, "Spirit", P0), 2,
        "Parallel Lives should double tokens");
}

/// Without Parallel Lives, token creation is normal.
#[test]
fn no_parallel_lives_single_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
        &reg,
    );

    assert_eq!(count_tokens_named_by(&state, "Spirit", P0), 1,
        "Without Parallel Lives, only one token");
}

/// Parallel Lives only doubles tokens for its controller.
#[test]
fn parallel_lives_only_doubles_for_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 controls Parallel Lives.
    let _pl = named_creature(&mut state, &reg, "Parallel Lives", P0);

    // P1 creates a token — should NOT be doubled.
    state.create_token(
        "Zombie", P1, 2, 2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        &reg,
    );

    assert_eq!(count_tokens_named_by(&state, "Zombie", P1), 1,
        "Parallel Lives shouldn't double opponent's tokens");
}

// ── Heartless Summoning ──────────────────────────────────────

/// Heartless Summoning gives creatures -1/-1.
#[test]
fn heartless_summoning_gives_minus_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Heartless Summoning on the battlefield.
    let _hs = named_creature(&mut state, &reg, "Heartless Summoning", P0);

    // Put a 3/3 creature on the battlefield.
    let creature = named_creature(&mut state, &reg, "Kindercatch", P0); // 6/6

    // Should be 5/5 with -1/-1.
    assert_eq!(state.effective_power(creature, &reg).unwrap(), 5);
    assert_eq!(state.effective_toughness(creature, &reg).unwrap(), 5);
}

/// Heartless Summoning reduces creature spell cost by {2}.
#[test]
fn heartless_summoning_reduces_creature_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Heartless Summoning on the battlefield.
    let _hs = named_creature(&mut state, &reg, "Heartless Summoning", P0);

    // Put a 6-mana creature in hand: Kindercatch costs {3}{G}{G}{G}.
    let spell = spell_in_hand(&mut state, &reg, "Kindercatch", P0);

    // With Heartless Summoning, it costs {1}{G}{G}{G} (3 generic reduced by 2).
    // Add only 4 mana (1 colorless + 3 green).
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 3);

    // Should be able to cast it.
    let legal = engine::legal_actions(&state, &reg);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell));
    assert!(can_cast, "Should be able to cast Kindercatch with reduced cost");
}

/// Heartless Summoning doesn't reduce non-creature costs.
#[test]
fn heartless_summoning_no_reduce_noncreature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Heartless Summoning on the battlefield.
    let _hs = named_creature(&mut state, &reg, "Heartless Summoning", P0);

    // Put an instant in hand: Lightning Bolt costs {R}.
    let spell = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);

    // Without red mana, shouldn't be able to cast it (no reduction for instants).
    let legal = engine::legal_actions(&state, &reg);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell));
    assert!(!can_cast, "Heartless Summoning shouldn't reduce instant cost");
}

// ── Rooftop Storm ──────────────────────────────────────────

/// Rooftop Storm makes Zombie creatures free.
#[test]
fn rooftop_storm_makes_zombies_free() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Rooftop Storm on the battlefield.
    let _rs = named_creature(&mut state, &reg, "Rooftop Storm", P0);

    // Put Walking Corpse (Zombie, {1}{B}) in hand.
    let spell = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);

    // Don't add any mana. Should still be castable (cost is {0}).
    let legal = engine::legal_actions(&state, &reg);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell));
    assert!(can_cast, "Rooftop Storm should make Zombie creatures free");
}

/// Rooftop Storm doesn't affect non-Zombie creatures.
#[test]
fn rooftop_storm_no_free_non_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Rooftop Storm on the battlefield.
    let _rs = named_creature(&mut state, &reg, "Rooftop Storm", P0);

    // Put a non-Zombie creature in hand: Grizzly Bears ({1}{G}).
    let spell = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    // Without mana, shouldn't be castable.
    let legal = engine::legal_actions(&state, &reg);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell));
    assert!(!can_cast, "Rooftop Storm shouldn't affect non-Zombie creatures");
}

// ── Nevermore ──────────────────────────────────────────

/// Nevermore prevents casting the named spell.
#[test]
fn nevermore_prevents_named_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a Nevermore for P0 that names "Lightning Bolt".
    let nevermore = named_creature(&mut state, &reg, "Nevermore", P0);
    if let Some(obj) = state.get_object_mut(nevermore) {
        obj.instance_continuous_effects = Some(vec![
            ContinuousEffect::PreventCastingNamed { name: "Lightning Bolt".into() },
        ]);
    }

    // Put Lightning Bolt in P1's hand with mana.
    state.priority_player = Some(P1);
    state.active_player = P1;
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);
    state.get_player_mut(P1).mana_pool.add(ManaType::Red, 1);

    // P1 should NOT be able to cast Lightning Bolt.
    let legal = engine::legal_actions(&state, &reg);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == bolt));
    assert!(!can_cast, "Nevermore should prevent casting the named spell");
}

/// Nevermore doesn't prevent other spells.
#[test]
fn nevermore_allows_other_spells() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);

    // Create a Nevermore for P0 that names "Lightning Bolt".
    let nevermore = named_creature(&mut state, &reg, "Nevermore", P0);
    if let Some(obj) = state.get_object_mut(nevermore) {
        obj.instance_continuous_effects = Some(vec![
            ContinuousEffect::PreventCastingNamed { name: "Lightning Bolt".into() },
        ]);
    }

    // Put Giant Growth in P1's hand with mana.
    let growth = spell_in_hand(&mut state, &reg, "Giant Growth", P1);
    state.get_player_mut(P1).mana_pool.add(ManaType::Green, 1);

    // Put a creature target for Giant Growth.
    let _creature = ready_creature(&mut state, P1, 2, 2);

    // P1 should be able to cast Giant Growth (not named by Nevermore).
    let legal = engine::legal_actions(&state, &reg);
    let can_cast = legal.actions.iter().any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == growth));
    assert!(can_cast, "Nevermore should not prevent other spells");
}

// ── Devil's Play ──────────────────────────────────────────

/// Devil's Play deals X damage.
#[test]
fn devils_play_deals_x_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Devil's Play in hand.
    let spell = spell_in_hand(&mut state, &reg, "Devil's Play", P0);

    // Add 4 mana: 3 colorless + 1 red. X should be 3.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    // Cast targeting opponent.
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    // Opponent should have taken 3 damage (20 - 3 = 17).
    assert_eq!(state.get_player(P1).life, 17, "Devil's Play should deal X=3 damage");
}

/// Devil's Play with X=0 deals no damage.
#[test]
fn devils_play_x_zero() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spell = spell_in_hand(&mut state, &reg, "Devil's Play", P0);

    // Add only 1 red mana. X should be 0.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    // Opponent should still be at 20.
    assert_eq!(state.get_player(P1).life, 20, "Devil's Play with X=0 should deal 0 damage");
}

// ── Kessig Wolf Run ──────────────────────────────────────────

/// Kessig Wolf Run with {1}{R}{G} funds X = 1, granting +1/+0 and trample.
#[test]
fn kessig_wolf_run_grants_power_and_trample() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    // {1}{R}{G} in the pool: {R}{G} pays the non-X portion, leaving 1
    // colorless that we'll allocate to X via the funding prompt.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let activated = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: wolf_run,
            ability_index: 1,
            targets: vec![Target::Object(creature)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        },
        &reg,
    );
    let new_state = resolve_funding_max(&activated, &reg);

    // Creature should have +1/+0 until end of turn (base 3/3 → 4/3).
    assert_eq!(new_state.effective_power(creature, &reg).unwrap(), 4);
    assert_eq!(new_state.effective_toughness(creature, &reg).unwrap(), 3);

    // Creature should have trample.
    assert!(new_state.has_keyword(creature, Keyword::Trample, &reg));
}

/// Kessig Wolf Run taps for colorless mana.
#[test]
fn kessig_wolf_run_taps_for_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);

    // Activate mana ability.
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility {
            object_id: wolf_run,
            ability_index: 0,
        },
        &reg,
    );

    assert_eq!(new_state.get_player(P0).mana_pool.get(ManaType::Colorless), 1);
}

// ── Snapcaster Mage ──────────────────────────────────────────

/// Snapcaster Mage grants flashback to an instant/sorcery in graveyard.
#[test]
fn snapcaster_mage_grants_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Lightning Bolt in P0's graveyard.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Graveyard, &reg);

    // Cast Snapcaster Mage (resolve immediately for ETB trigger).
    let snap = castable_spell(&mut state, &reg, "Snapcaster Mage", P0);
    let mut new_state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: snap, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &reg);

    // Process ETB triggers.
    mtg_engine::triggers::collect_triggers(&mut new_state, &reg);
    // Triggers go on the stack; resolve them.
    while new_state.stack.last().is_some_and(|e| matches!(e, mtg_engine::state::StackEntry::Trigger(_))) {
        mtg_engine::triggers::resolve_next_trigger(&mut new_state, &reg);
    }

    // Lightning Bolt should have flashback granted.
    let has_flashback = new_state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == bolt));
    assert!(has_flashback, "Snapcaster Mage should grant flashback to Lightning Bolt");
}

// ── Past in Flames ──────────────────────────────────────────

/// Past in Flames grants flashback to all instants/sorceries in graveyard.
#[test]
fn past_in_flames_grants_flashback_to_all() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put some instants/sorceries in P0's graveyard.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Graveyard, &reg);

    let divination = spell_in_hand(&mut state, &reg, "Divination", P0);
    state.move_object(divination, Zone::Graveyard, &reg);

    // Also put a creature in graveyard (should NOT get flashback).
    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(creature, Zone::Graveyard, &reg);

    // Cast Past in Flames.
    let spell = castable_spell(&mut state, &reg, "Past in Flames", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Lightning Bolt and Divination should have flashback.
    let bolt_fb = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == bolt));
    let div_fb = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == divination));
    let creature_fb = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == creature));

    assert!(bolt_fb, "Past in Flames should grant flashback to Lightning Bolt");
    assert!(div_fb, "Past in Flames should grant flashback to Divination");
    assert!(!creature_fb, "Past in Flames should NOT grant flashback to creatures");
}

// ── Olivia Voldaren ──────────────────────────────────────────

/// Olivia's first ability deals 1 damage, makes target a Vampire, and gets a +1/+1 counter.
#[test]
fn olivia_ping_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_creature(&mut state, &reg, "Olivia Voldaren", P0);
    let target_creature = ready_creature(&mut state, P1, 2, 3);
    state.get_object_mut(target_creature).unwrap().name = "Test Creature".into();

    // Add mana for ability: {1}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: olivia,
            ability_index: 0,
            targets: vec![Target::Object(target_creature)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        },
        &reg,
    );

    // Target should have 1 damage.
    assert_eq!(new_state.get_object(target_creature).unwrap().damage_marked, 1);

    // Target should be a Vampire.
    assert!(new_state.get_object(target_creature).unwrap().subtypes.contains(&"Vampire".to_string()));

    // Olivia should have a +1/+1 counter.
    assert_eq!(counters_of(&new_state, olivia, CounterType::PlusOnePlusOne), 1);
}

/// Olivia's second ability gains control of a Vampire.
#[test]
fn olivia_gain_control_of_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_creature(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(vampire).unwrap().subtypes = vec!["Vampire".into()];
    state.get_object_mut(vampire).unwrap().name = "Enemy Vampire".into();

    // Add mana for ability: {3}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: olivia,
            ability_index: 1,
            targets: vec![Target::Object(vampire)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        },
        &reg,
    );

    // P0 should now control the vampire.
    assert_eq!(new_state.get_object(vampire).unwrap().controller, P0);
}

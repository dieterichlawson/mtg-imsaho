//! AI scenario tests for Tier 4 flashback cards.
//!
//! Each test builds a game state where the correct play is obvious,
//! then verifies Claude makes the right decision.
//!
//! For flashback tests, the card is placed in the graveyard (not hand)
//! with enough mana for the flashback cost. The action appears as
//! "Flashback X" in the LLM prompt.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_tier4
//! Requires ANTHROPIC_API_KEY to be set.

use std::fs;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::state::{AwaitingAction, CombatState, GameState};
use mtg_engine::types::*;
use mtg_engine::view::GameView;

use mtg_player::llm::LlmPlayer;
use mtg_player::Player;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SaveData {
    state: GameState,
    player_names: Vec<String>,
}

fn save_scenario(state: &GameState, name: &str) {
    let save = SaveData {
        state: state.clone(),
        player_names: vec!["Opponent".into(), "AI".into()],
    };
    let path = format!("/tmp/{}.json", name);
    let json = serde_json::to_string_pretty(&save).unwrap();
    fs::write(&path, &json).unwrap();
    eprintln!("Saved scenario to {}", path);
}

fn add_libraries(state: &mut GameState, registry: &CardRegistry) {
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let swamp_id = registry.get_id_by_name("Swamp").unwrap();
    for p in 0..2u8 {
        let land_id = if p == 0 { forest_id } else { swamp_id };
        let name = if p == 0 { "Forest" } else { "Swamp" };
        let mut lib = Vec::new();
        for _ in 0..15 {
            let id = state.create_object(land_id, PlayerId(p), Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = name.into();
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }
}

/// Run the AI decision loop. When the AI casts a spell, submits the action
/// and resolves the stack, then returns the action and the post-resolution state.
/// Handles mana tapping automatically.
fn run_ai_decision(
    state: &GameState,
    player_id: PlayerId,
    player: &mut LlmPlayer,
    registry: &CardRegistry,
) -> (Action, GameState) {
    let mut current = state.clone();
    for i in 0..15 {
        let legal = engine::legal_actions(&current, registry);
        if legal.combat_prompt.is_some() {
            let view = GameView::for_player(&current, player_id, registry);
            let action = player.choose_combat(&view, legal.combat_prompt.as_ref().unwrap());
            return (action, current);
        }
        let view = GameView::for_player(&current, player_id, registry);
        let action = player.choose_action(&view, &legal);
        match &action {
            Action::CastSpell { .. } => {
                eprintln!("  AI cast spell on action #{}", i + 1);
                // Submit the cast action and resolve the stack.
                current = engine::submit_action(&current, &action, registry);
                mtg_engine::stack::resolve_top_of_stack(&mut current, registry);
                mtg_engine::sba::check_state_based_actions_with_registry(&mut current, Some(registry));
                mtg_engine::triggers::process_triggers(&mut current, registry);

                // Handle any resolution choice set by the spell/trigger.
                while let Some(AwaitingAction::ResolutionChoice { player: choice_player, .. }) = &current.awaiting_action {
                    if *choice_player == player_id {
                        // AI's choice -- make another API call.
                        let choice_legal = engine::legal_actions(&current, registry);
                        let choice_view = GameView::for_player(&current, player_id, registry);
                        let choice_action = player.choose_action(&choice_view, &choice_legal);
                        eprintln!("  AI made resolution choice");
                        current = engine::submit_action(&current, &choice_action, registry);
                        // Continue processing SBAs/triggers after the choice.
                        mtg_engine::sba::check_state_based_actions_with_registry(&mut current, Some(registry));
                        mtg_engine::triggers::process_triggers(&mut current, registry);
                    } else {
                        // Opponent's choice -- return to test to handle deterministically.
                        break;
                    }
                }

                return (action, current);
            }
            Action::ActivateAbility { object_id, .. } => {
                let name = current.get_object(*object_id)
                    .map(|o| o.name.as_str()).unwrap_or("?");
                eprintln!("  AI activated ability on {} at action #{}", name, i + 1);
                current = engine::submit_action(&current, &action, registry);
                return (action, current);
            }
            Action::ActivateManaAbility { object_id, .. } => {
                let name = current.get_object(*object_id)
                    .map(|o| o.name.as_str()).unwrap_or("?");
                eprintln!("  Action {}: Tapped {} for mana", i + 1, name);
                current = engine::submit_action(&current, &action, registry);
            }
            Action::PassPriority => {
                return (action, current);
            }
            other => {
                return (other.clone(), current);
            }
        }
    }
    panic!("AI did not act within 15 actions");
}


fn spell_name<'a>(state: &'a GameState, action: &Action) -> &'a str {
    match action {
        Action::CastSpell { object_id, .. } => {
            state.get_object(*object_id).map(|o| o.name.as_str()).unwrap_or("?")
        }
        _ => "?",
    }
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 1: Flashback Think Twice from graveyard
//
// P0 (AI) main phase, Think Twice in graveyard, 3 Islands.
// Flashback cost is {2}{U}. Drawing a card is always good.
// Should flashback it.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_think_twice_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 20;
    state.turn_number = 8;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Think Twice in graveyard
    let tt_id = reg.get_id_by_name("Think Twice").unwrap();
    let tt = state.create_object(tt_id, PlayerId(0), Zone::Graveyard, None, None);
    state.get_object_mut(tt).unwrap().name = "Think Twice".into();

    // P0 (AI): 3 Islands for {2}{U} flashback cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_think_twice_fb");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_think_twice_fb.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should flashback Think Twice, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Think Twice");
    // Verify outcome: drew a card and spell is exiled (flashback).
    let hand_size = final_state.objects_in_zone(Zone::Hand, PlayerId(0)).len();
    assert!(hand_size >= 1, "Should have drawn a card, hand size = {}", hand_size);
    assert_eq!(final_state.get_object(tt).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled after resolution");
    eprintln!("OK: AI flashed back Think Twice — drew a card, spell exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 2: Flashback Geistflame for lethal
//
// P0 (AI), opponent at 1 life, Geistflame in graveyard, 4 Mountains.
// Flashback cost is {3}{R}. Should flashback for lethal.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_geistflame_flashback_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 1;
    state.turn_number = 12;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Geistflame in graveyard
    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let gf = state.create_object(gf_id, PlayerId(0), Zone::Graveyard, None, None);
    state.get_object_mut(gf).unwrap().name = "Geistflame".into();

    // P0 (AI): 4 Mountains for {3}{R} flashback cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..4 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_geistflame_fb_lethal");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_geistflame_fb_lethal.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should flashback Geistflame for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Geistflame");
    if let Action::CastSpell { targets, .. } = &action {
        assert!(targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == PlayerId(1))),
            "Should target opponent for lethal damage");
    }
    // Verify outcome: opponent should be at 0 or less life.
    assert!(final_state.get_player(PlayerId(1)).life <= 0,
        "Geistflame should deal 1 damage for lethal, opponent life = {}", final_state.get_player(PlayerId(1)).life);
    eprintln!("OK: AI flashed back Geistflame at opponent for lethal (life={})", final_state.get_player(PlayerId(1)).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 3: Flashback Bump in the Night for lethal
//
// P0 (AI), opponent at 3 life, Bump in graveyard. Flashback cost
// is {5}{R} = 6 mana (5 generic + 1 red). 6 Mountains suffice.
// Should flashback for lethal (opponent loses 3 life).
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_bump_flashback_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 3;
    state.turn_number = 14;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Bump in the Night in graveyard
    let bump_id = reg.get_id_by_name("Bump in the Night").unwrap();
    let bump = state.create_object(bump_id, PlayerId(0), Zone::Graveyard, None, None);
    state.get_object_mut(bump).unwrap().name = "Bump in the Night".into();

    // P0 (AI): 6 Mountains for {5}{R} flashback cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..6 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_bump_fb_lethal");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_bump_fb_lethal.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should flashback Bump in the Night for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bump in the Night");
    // Verify outcome: opponent loses 3 life (was at 3, now 0 or less).
    assert!(final_state.get_player(PlayerId(1)).life <= 0,
        "Bump should drain 3 for lethal, opponent life = {}", final_state.get_player(PlayerId(1)).life);
    eprintln!("OK: AI flashed back Bump in the Night for lethal (life={})", final_state.get_player(PlayerId(1)).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 4: Flashback Silent Departure to bounce a threat
//
// P1 (AI) at 6 life, opponent has 5/5, Silent Departure in graveyard,
// 5 Islands. Flashback cost is {4}{U}. Should flashback to bounce
// the threatening creature.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_silent_departure_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 10;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: big 5/5 creature
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let big = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(5), Some(5));
    state.get_object_mut(big).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().colors = vec![Color::Green];

    // P1 (AI): Silent Departure in graveyard
    let sd_id = reg.get_id_by_name("Silent Departure").unwrap();
    let sd = state.create_object(sd_id, PlayerId(1), Zone::Graveyard, None, None);
    state.get_object_mut(sd).unwrap().name = "Silent Departure".into();

    // P1 (AI): 5 Islands for {4}{U} flashback cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..5 {
        let id = state.create_object(island_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_silent_departure_fb");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_silent_departure_fb.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should flashback Silent Departure, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Silent Departure");
    // Verify outcome: 5/5 should be bounced to hand, spell exiled.
    assert_eq!(final_state.get_object(big).unwrap().zone, Zone::Hand,
        "Silent Departure should bounce the 5/5 to hand");
    assert_eq!(final_state.get_object(sd).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled");
    eprintln!("OK: AI flashed back Silent Departure — 5/5 bounced, spell exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 5: Cast Dream Twist to mill out opponent
//
// P0 (AI) main phase, Dream Twist in hand, 1 Island. Opponent has
// only 3 cards left in library. Milling 3 empties it — opponent
// loses on their next draw step. This is lethal!
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_dream_twist() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 5;
    state.players[1].life = 20;
    state.turn_number = 20;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Dream Twist in hand
    let dt_id = reg.get_id_by_name("Dream Twist").unwrap();
    let dt = state.create_object(dt_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(dt).unwrap().name = "Dream Twist".into();

    // P0 (AI): 1 Island
    let island_id = reg.get_id_by_name("Island").unwrap();
    let isl = state.create_object(island_id, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(isl).unwrap().name = "Island".into();
    state.get_object_mut(isl).unwrap().summoning_sick = false;

    // P0 library: 15 cards (healthy)
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut p0_lib = Vec::new();
    for _ in 0..15 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        p0_lib.push(id);
    }
    state.players[0].library_order = p0_lib;

    // P1 library: only 3 cards left — milling 3 empties it!
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut p1_lib = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(swamp_id, PlayerId(1), Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        p1_lib.push(id);
    }
    state.players[1].library_order = p1_lib;

    save_scenario(&state, "ai_dream_twist");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_dream_twist.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Dream Twist to mill out opponent, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Dream Twist");
    // Verify outcome: opponent's library is empty.
    assert_eq!(final_state.get_player(PlayerId(1)).library_order.len(), 0,
        "Dream Twist should empty opponent's library (3 cards milled)");
    eprintln!("OK: AI cast Dream Twist — opponent's library is empty, they lose on next draw");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 6: Cast Travel Preparations to buff a creature
//
// P0 (AI) main phase, creature on battlefield, Travel Preparations
// in hand, 2 Forests. Cost is {1}{G}. Should cast to add a +1/+1
// counter to the creature.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_travel_preparations() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 15;
    state.turn_number = 4;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): two creatures on battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears1 = state.create_object(bears_id, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(bears1).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(bears1).unwrap().summoning_sick = false;
    state.get_object_mut(bears1).unwrap().colors = vec![Color::Green];
    state.get_object_mut(bears1).unwrap().controller = PlayerId(0);

    let viper_id = reg.get_id_by_name("Ambush Viper").unwrap();
    let viper = state.create_object(viper_id, PlayerId(0), Zone::Battlefield, Some(2), Some(1));
    state.get_object_mut(viper).unwrap().name = "Ambush Viper".into();
    state.get_object_mut(viper).unwrap().summoning_sick = false;
    state.get_object_mut(viper).unwrap().colors = vec![Color::Green];
    state.get_object_mut(viper).unwrap().controller = PlayerId(0);

    // P0 (AI): Travel Preparations in hand
    let tp_id = reg.get_id_by_name("Travel Preparations").unwrap();
    let tp = state.create_object(tp_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(tp).unwrap().name = "Travel Preparations".into();

    // P0 (AI): 2 Forests for {1}{G} cost
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..2 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_travel_preparations");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_travel_preparations.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Travel Preparations, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Travel Preparations");
    // Verify outcome: at least one creature got a counter, ideally both.
    let bears_counters = final_state.get_counter_count(bears1, CounterType::PlusOnePlusOne);
    let viper_counters = final_state.get_counter_count(viper, CounterType::PlusOnePlusOne);
    let total_counters = bears_counters + viper_counters;
    assert!(total_counters >= 1, "Travel Preparations should add at least one +1/+1 counter");
    eprintln!("OK: AI cast Travel Preparations — {} counters placed (bears={}, viper={})",
        total_counters, bears_counters, viper_counters);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 7: Cast Rolling Temblor to wipe opponent's creatures
//
// P0 (AI) has Rolling Temblor in hand + 3 Mountains. Cost is {2}{R}.
// Opponent has two 2/2 ground creatures. AI has no creatures.
// Should cast to deal 2 damage to each, killing both.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_rolling_temblor() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 12;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: two 2/2 ground creatures
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..2 {
        let creature = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(creature).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(creature).unwrap().summoning_sick = false;
        state.get_object_mut(creature).unwrap().colors = vec![Color::Green];
        state.get_object_mut(creature).unwrap().controller = PlayerId(1);
    }

    // P0 (AI): Rolling Temblor in hand
    let rt_id = reg.get_id_by_name("Rolling Temblor").unwrap();
    let rt = state.create_object(rt_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(rt).unwrap().name = "Rolling Temblor".into();

    // P0 (AI): 3 Mountains for {2}{R} cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..3 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_rolling_temblor");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_rolling_temblor.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Rolling Temblor to kill both 2/2s, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rolling Temblor");
    // Verify outcome: opponent's 2/2s should have 2 damage (lethal after SBA).
    let opp_creatures: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(1))
        .iter().filter(|o| o.power.is_some()).map(|o| o.id).collect();
    assert_eq!(opp_creatures.len(), 0,
        "Rolling Temblor should kill both 2/2 ground creatures");
    eprintln!("OK: AI cast Rolling Temblor — both 2/2s killed");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 8: Cast Unburial Rites to reanimate a creature
//
// P0 (AI), TWO creatures in graveyard (Kindercatch 6/6 and Grizzly
// Bears 2/2), Unburial Rites in hand, 5 Swamps. Cost is {4}{B}.
// With 2+ targets, the choice system triggers and the AI picks one.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_unburial_rites() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 20;
    state.turn_number = 7;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): TWO creatures in graveyard so the choice system triggers
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, PlayerId(0), Zone::Graveyard, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];

    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, PlayerId(0), Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(bears).unwrap().colors = vec![Color::Green];

    // P0 (AI): Unburial Rites in hand
    let ur_id = reg.get_id_by_name("Unburial Rites").unwrap();
    let ur = state.create_object(ur_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(ur).unwrap().name = "Unburial Rites".into();

    // P0 (AI): 5 Swamps for {4}{B} cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    for _ in 0..5 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_unburial_rites");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_unburial_rites.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Unburial Rites to reanimate, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Unburial Rites");
    // Verify outcome: one creature on the battlefield (AI chose which to reanimate).
    let kc_zone = final_state.get_object(kc).unwrap().zone;
    let bears_zone = final_state.get_object(bears).unwrap().zone;
    assert!(kc_zone == Zone::Battlefield || bears_zone == Zone::Battlefield,
        "Unburial Rites should return one creature to the battlefield (Kindercatch={:?}, Bears={:?})",
        kc_zone, bears_zone);
    eprintln!("OK: AI cast Unburial Rites — reanimated creature (Kindercatch={:?}, Bears={:?})",
        kc_zone, bears_zone);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 9: Cast Gnaw to the Bone to gain life
//
// P0 (AI) at 3 life with 4 creature cards in graveyard. Gnaw in
// hand, 3 Forests. Cost is {2}{G}. Should cast to gain 8 life
// (2 per creature card in graveyard).
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_gnaw_to_the_bone() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 3;
    state.players[1].life = 20;
    state.turn_number = 9;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): 4 creature cards in graveyard
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..4 {
        let creature = state.create_object(bears_id, PlayerId(0), Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(creature).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(creature).unwrap().colors = vec![Color::Green];
    }

    // P0 (AI): Gnaw to the Bone in hand
    let gnaw_id = reg.get_id_by_name("Gnaw to the Bone").unwrap();
    let gnaw = state.create_object(gnaw_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(gnaw).unwrap().name = "Gnaw to the Bone".into();

    // P0 (AI): 3 Forests for {2}{G} cost
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..3 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_gnaw_to_the_bone");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_gnaw_to_the_bone.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Gnaw to the Bone at 3 life, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Gnaw to the Bone");
    // Verify outcome: AI started at 3 life, should gain 2 * 4 = 8 life → 11.
    assert!(final_state.get_player(PlayerId(0)).life > 3,
        "Gnaw should gain life, AI life = {}", final_state.get_player(PlayerId(0)).life);
    eprintln!("OK: AI cast Gnaw to the Bone — life now {}", final_state.get_player(PlayerId(0)).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 10: Cast Desperate Ravings for card advantage
//
// P0 (AI) main phase, no other plays, Desperate Ravings in hand,
// 2 Mountains. Cost is {1}{R}. Should cast for card advantage
// (draw 2, discard 1 random).
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_desperate_ravings() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 15;
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Desperate Ravings in hand
    let dr_id = reg.get_id_by_name("Desperate Ravings").unwrap();
    let dr = state.create_object(dr_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(dr).unwrap().name = "Desperate Ravings".into();

    // P0 (AI): 2 Mountains for {1}{R} cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..2 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_desperate_ravings");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_desperate_ravings.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Desperate Ravings for card advantage, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Desperate Ravings");
    // Verify outcome: net +1 card in hand (draw 2, discard 1, cast 1 = started with 1, now should have 1).
    // AI had Desperate Ravings in hand (1 card). Cast it (0), drew 2, discarded 1 → 1 card.
    let hand = final_state.objects_in_zone(Zone::Hand, PlayerId(0)).len();
    assert!(hand >= 1, "Should have cards in hand after draw 2 discard 1, hand = {}", hand);
    eprintln!("OK: AI cast Desperate Ravings — hand size {}", hand);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 11: Cast Forbidden Alchemy for card selection
//
// P0 (AI) main phase, Forbidden Alchemy in hand, 3 Islands.
// Cost is {2}{U}. The library has 15 cards. After resolve, 4 cards
// are revealed and the AI chooses which to keep (via ResolutionChoice).
// Verify 1 card in hand and the rest in graveyard.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_forbidden_alchemy() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 15;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Forbidden Alchemy in hand
    let fa_id = reg.get_id_by_name("Forbidden Alchemy").unwrap();
    let fa = state.create_object(fa_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(fa).unwrap().name = "Forbidden Alchemy".into();

    // P0 (AI): 3 Islands for {2}{U} cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    let lib_before = state.get_player(PlayerId(0)).library_order.len();
    save_scenario(&state, "ai_forbidden_alchemy");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_forbidden_alchemy.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Forbidden Alchemy for card selection, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Forbidden Alchemy");

    // Verify outcome: AI chose 1 card to keep (in hand), 3 went to graveyard.
    // Hand had Forbidden Alchemy (cast it → 0 cards), then 1 chosen → 1 in hand.
    let hand_size = final_state.objects_in_zone(Zone::Hand, PlayerId(0)).len();
    assert!(hand_size >= 1, "AI should have chosen 1 card to keep, hand = {}", hand_size);
    // Library should shrink by 4 (4 revealed, 1 kept, 3 milled).
    let lib_after = final_state.get_player(PlayerId(0)).library_order.len();
    assert_eq!(lib_before - lib_after, 4,
        "Should have removed 4 cards from library (before={}, after={})", lib_before, lib_after);
    eprintln!("OK: AI cast Forbidden Alchemy — chose 1 card, library {} → {}", lib_before, lib_after);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 12: Cast Feeling of Dread to tap opponent's creature
//
// P1 (AI) at 6 life. P0 has untapped 5/5 creature. P1 has Feeling
// of Dread in hand + 2 Plains. Cost is {1}{W}. Should cast to tap
// the threatening creature before it can attack.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_feeling_of_dread() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 7;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: two threatening creatures (combined damage is lethal)
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let big1 = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(big1).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big1).unwrap().summoning_sick = false;
    state.get_object_mut(big1).unwrap().colors = vec![Color::Green];

    let big2 = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(big2).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big2).unwrap().summoning_sick = false;
    state.get_object_mut(big2).unwrap().colors = vec![Color::Green];

    // P1 (AI): Feeling of Dread in hand
    let fod_id = reg.get_id_by_name("Feeling of Dread").unwrap();
    let fod = state.create_object(fod_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(fod).unwrap().name = "Feeling of Dread".into();

    // P1 (AI): 2 Plains for {1}{W} cost
    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..2 {
        let id = state.create_object(plains_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_feeling_of_dread");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_feeling_of_dread.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Feeling of Dread to tap opponent's creatures, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Feeling of Dread");
    // Verify outcome: both opponent creatures should be tapped (UpToTargets(2)).
    assert!(final_state.get_object(big1).unwrap().tapped,
        "Feeling of Dread should tap the first creature");
    assert!(final_state.get_object(big2).unwrap().tapped,
        "Feeling of Dread should tap the second creature");
    eprintln!("OK: AI cast Feeling of Dread — both creatures are now tapped");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 13: Nightbird's Clutches to clear blocker for lethal
//
// P0 (AI) has a 3/3, opponent at 3 life with a 3/3 blocker.
// If the blocker is tapped, the 3/3 attacks unblocked for lethal.
// Nightbird's Clutches is the only card in hand. No other play wins.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_nightbirds_clutches() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 3; // 3/3 unblocked = lethal
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): 3/3 creature ready to attack
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let attacker = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(attacker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(attacker).unwrap().summoning_sick = false;
    state.get_object_mut(attacker).unwrap().colors = vec![Color::Green];
    state.get_object_mut(attacker).unwrap().controller = PlayerId(0);

    // P1: 3/3 blocker that would trade
    let blocker = state.create_object(tusker_id, PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(blocker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(blocker).unwrap().summoning_sick = false;
    state.get_object_mut(blocker).unwrap().colors = vec![Color::Green];
    state.get_object_mut(blocker).unwrap().controller = PlayerId(1);

    // P0 (AI): Nightbird's Clutches in hand
    let nc_id = reg.get_id_by_name("Nightbird's Clutches").unwrap();
    let nc = state.create_object(nc_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(nc).unwrap().name = "Nightbird's Clutches".into();

    // P0 (AI): 2 Mountains for {1}{R} cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..2 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_nightbirds_clutches");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_nightbirds_clutches.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Nightbird's Clutches to clear the blocker, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Nightbird's Clutches");
    // Verify outcome: opponent's blocker can't block this turn.
    assert!(final_state.until_end_of_turn_cant_block.contains(&blocker),
        "Nightbird's Clutches should prevent the creature from blocking this turn");
    let eligible = combat::eligible_blockers_with_registry(&final_state, PlayerId(1), &reg);
    assert!(!eligible.contains(&blocker),
        "Blocker should not appear in eligible blockers after Nightbird's Clutches");
    eprintln!("OK: AI cast Nightbird's Clutches — blocker can't block this turn");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 14: Flashback Rally the Peasants to pump creatures
//
// P0 (AI) has two creatures and Rally the Peasants in graveyard.
// 3 Mountains for {2}{R} flashback cost. Should flashback to give
// creatures +2/+0 before combat for lethal.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_rally_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 5;
    state.turn_number = 8;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): two 2/2 creatures ready to attack
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..2 {
        let creature = state.create_object(bears_id, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(creature).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(creature).unwrap().summoning_sick = false;
        state.get_object_mut(creature).unwrap().colors = vec![Color::Green];
        state.get_object_mut(creature).unwrap().controller = PlayerId(0);
    }

    // P0 (AI): Rally the Peasants in graveyard
    let rally_id = reg.get_id_by_name("Rally the Peasants").unwrap();
    let rally = state.create_object(rally_id, PlayerId(0), Zone::Graveyard, None, None);
    state.get_object_mut(rally).unwrap().name = "Rally the Peasants".into();

    // P0 (AI): 3 Mountains for {2}{R} flashback cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..3 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_rally_fb");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_rally_fb.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should flashback Rally the Peasants, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rally the Peasants");
    // Verify outcome: creatures should have +2/+0 (effective 4/2), spell exiled.
    let my_creatures: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .iter().filter(|o| o.power.is_some()).map(|o| o.id).collect();
    for &id in &my_creatures {
        assert_eq!(final_state.effective_power(id, &reg), Some(4),
            "Creatures should be pumped to 4 power from Rally");
    }
    assert_eq!(final_state.get_object(rally).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled");
    eprintln!("OK: AI flashed back Rally — creatures pumped to 4/2, spell exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 15: Flashback Moan of the Unhallowed for zombie tokens
//
// P0 (AI) needs creatures. Moan in graveyard, 7 Swamps for
// {5}{B}{B} flashback cost. Should flashback for two 2/2 Zombies.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_moan_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 15;
    state.turn_number = 12;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: a creature threatening AI
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0 (AI): Moan of the Unhallowed in graveyard
    let moan_id = reg.get_id_by_name("Moan of the Unhallowed").unwrap();
    let moan = state.create_object(moan_id, PlayerId(0), Zone::Graveyard, None, None);
    state.get_object_mut(moan).unwrap().name = "Moan of the Unhallowed".into();

    // P0 (AI): 7 Swamps for {5}{B}{B} flashback cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    for _ in 0..7 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_moan_fb");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_moan_fb.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should flashback Moan of the Unhallowed, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Moan of the Unhallowed");
    // Verify outcome: two 2/2 Zombie tokens on the battlefield, spell exiled.
    let my_creatures: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .iter().filter(|o| o.power.is_some()).map(|o| o.id).collect();
    assert_eq!(my_creatures.len(), 2, "Should have two Zombie tokens on battlefield");
    assert_eq!(final_state.get_object(moan).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled");
    eprintln!("OK: AI flashed back Moan — two Zombie tokens created, spell exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Skeletal Grimace regeneration in response to Doom Blade
//
// P0 has cast Doom Blade targeting P1's creature (on the stack).
// P1 (AI) has a 2/2 creature enchanted with Skeletal Grimace (+1/+1)
// and an untapped Swamp for {B}. P1 also has a Grizzly Bears in hand
// (uncastable — no green mana) so there are other actions available.
// Correct play: activate regeneration to save the creature.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_skeletal_grimace_regenerate() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 14;
    state.turn_number = 7;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(1)); // AI has priority to respond
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.players[1].land_plays_remaining = 0;

    // P1 (AI): a 2/2 creature on the battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // P1 (AI): Skeletal Grimace attached to creature
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_object_mut(sg).unwrap().attached_to = Some(creature);
    state.get_object_mut(sg).unwrap().summoning_sick = false;

    // P1 (AI): 1 Swamp (untapped) for {B} regeneration cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().summoning_sick = false;

    // P0: 2 Swamps (tapped, already used for Doom Blade)
    for _ in 0..2 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().tapped = true;
    }

    // P0 has cast Doom Blade targeting P1's creature — put it on the stack.
    let db_id = reg.get_id_by_name("Doom Blade").unwrap();
    let doom_blade = state.create_object(db_id, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(doom_blade).unwrap().name = "Doom Blade".into();
    state.get_object_mut(doom_blade).unwrap().targets = vec![mtg_engine::actions::Target::Object(creature)];
    state.stack.push(doom_blade);

    // Give P1 another card in hand (not castable) to give choices.
    let bear2 = state.create_object(bears_id, PlayerId(1), Zone::Hand, Some(2), Some(2));
    state.get_object_mut(bear2).unwrap().name = "Grizzly Bears".into();

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_skeletal_grimace_regen");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_skeletal_grimace_regen.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    // AI should activate regeneration ability.
    assert!(matches!(&action, Action::ActivateAbility { .. }),
        "AI should activate regenerate to save creature from Doom Blade, not {:?}", action);

    // Verify the creature now has a regeneration shield.
    assert_eq!(final_state.get_object(creature).unwrap().regeneration_shields, 1,
        "Creature should have a regeneration shield");

    // Now resolve Doom Blade — creature should survive via regeneration.
    let mut post_resolve = final_state.clone();
    // Pass priority for both players to let Doom Blade resolve.
    post_resolve.consecutive_passes = 2;
    mtg_engine::stack::resolve_top_of_stack(&mut post_resolve, &reg);
    mtg_engine::sba::check_state_based_actions_with_registry(&mut post_resolve, Some(&reg));

    assert_eq!(post_resolve.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive Doom Blade thanks to regeneration");
    assert!(post_resolve.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(post_resolve.get_object(creature).unwrap().regeneration_shields, 0,
        "Regeneration shield should be consumed");
    eprintln!("OK: AI activated regenerate → creature survived Doom Blade (tapped, shield consumed)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Skeletal Grimace regeneration in response to Lightning Bolt
//
// P0 has cast Lightning Bolt targeting P1's 2/2 creature enchanted
// with Skeletal Grimace (effective 3/3). Bolt deals 3 = lethal.
// P1 (AI) has priority to respond with an untapped Swamp for {B}.
// P1 also has a Grizzly Bears in hand (can't cast at instant speed).
// Correct play: activate regeneration — the creature survives the
// damage via the regeneration replacement in SBAs.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_skeletal_grimace_regen_vs_bolt() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 18;
    state.players[1].life = 12;
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(1)); // AI has priority to respond
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.players[1].land_plays_remaining = 0;

    // P1 (AI): a 2/2 creature on the battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // P1 (AI): Skeletal Grimace attached to creature
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_object_mut(sg).unwrap().attached_to = Some(creature);
    state.get_object_mut(sg).unwrap().summoning_sick = false;

    // P1 (AI): 1 Swamp (untapped) for {B} regeneration cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().summoning_sick = false;

    // P1 (AI): a Grizzly Bears in hand (sorcery-speed, can't help here)
    let bear2 = state.create_object(bears_id, PlayerId(1), Zone::Hand, Some(2), Some(2));
    state.get_object_mut(bear2).unwrap().name = "Grizzly Bears".into();

    // P0: 1 Mountain (tapped, used for Lightning Bolt)
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mtn = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(mtn).unwrap().name = "Mountain".into();
    state.get_object_mut(mtn).unwrap().tapped = true;

    // P0 has cast Lightning Bolt targeting P1's creature — on the stack.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_id, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(bolt).unwrap().name = "Lightning Bolt".into();
    state.get_object_mut(bolt).unwrap().targets = vec![mtg_engine::actions::Target::Object(creature)];
    state.stack.push(bolt);

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_skeletal_grimace_regen_bolt");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_skeletal_grimace_regen_bolt.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    // AI should activate regeneration to save the creature from lethal damage.
    assert!(matches!(&action, Action::ActivateAbility { .. }),
        "AI should activate regenerate to save creature from Lightning Bolt, not {:?}", action);

    // Verify the creature has a regeneration shield.
    assert_eq!(final_state.get_object(creature).unwrap().regeneration_shields, 1,
        "Creature should have a regeneration shield");

    // Resolve Lightning Bolt — deals 3 damage to the 3/3 creature.
    let mut post_resolve = final_state.clone();
    post_resolve.consecutive_passes = 2;
    mtg_engine::stack::resolve_top_of_stack(&mut post_resolve, &reg);
    // SBAs: 3 damage on 3 toughness = lethal → regeneration replaces destruction.
    mtg_engine::sba::check_state_based_actions_with_registry(&mut post_resolve, Some(&reg));

    assert_eq!(post_resolve.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive Lightning Bolt thanks to regeneration");
    assert!(post_resolve.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(post_resolve.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be removed by regeneration");
    assert_eq!(post_resolve.get_object(creature).unwrap().regeneration_shields, 0,
        "Regeneration shield should be consumed");
    eprintln!("OK: AI activated regenerate → creature survived Lightning Bolt (3 damage on 3/3, regenerated)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Skeletal Grimace regeneration vs deathtouch in combat
//
// P0's Typhoid Rats (1/1 deathtouch) is attacking. P1 (AI) has a 2/2
// creature enchanted with Skeletal Grimace (effective 3/3) and an
// untapped Swamp. P1 has priority during DeclareBlockers.
// Correct play: activate regeneration, then block with the creature.
// After combat damage, even 1 deathtouch damage is lethal, but
// regeneration saves the creature. Typhoid Rats dies.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier4_skeletal_grimace_regen_vs_deathtouch() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 10;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.players[1].land_plays_remaining = 0;

    // P0: Typhoid Rats (1/1 deathtouch) attacking
    let rats_id = reg.get_id_by_name("Typhoid Rats").unwrap();
    let rats = state.create_object(rats_id, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(rats).unwrap().name = "Typhoid Rats".into();
    state.get_object_mut(rats).unwrap().summoning_sick = false;
    state.get_object_mut(rats).unwrap().tapped = true;
    state.get_object_mut(rats).unwrap().colors = vec![Color::Black];
    state.get_object_mut(rats).unwrap().keywords = vec![Keyword::Deathtouch];

    let mut combat_state = CombatState::new();
    combat_state.attackers.insert(rats, PlayerId(1));
    combat_state.blocker_assignments.insert(rats, Vec::new());
    state.combat = Some(combat_state);

    // P1 (AI): 2/2 creature on the battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // P1 (AI): Skeletal Grimace attached to creature
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_object_mut(sg).unwrap().attached_to = Some(creature);
    state.get_object_mut(sg).unwrap().summoning_sick = false;

    // P1 (AI): 1 Swamp (untapped) for {B}
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().summoning_sick = false;

    // P1 has priority during DeclareBlockers.
    state.priority_player = Some(PlayerId(1));

    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Typhoid Rats".into());

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_skeletal_grimace_regen_deathtouch");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_skeletal_grimace_regen_deathtouch.log");

    // First decision: AI should activate regeneration.
    let (action, mut current) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::ActivateAbility { .. }),
        "AI should activate regenerate before blocking deathtouch creature, not {:?}", action);
    assert_eq!(current.get_object(creature).unwrap().regeneration_shields, 1,
        "Creature should have a regeneration shield");
    eprintln!("  AI activated regeneration shield");

    // Second decision: AI should declare blockers (block Rats with creature).
    // Pass priority to get to blocker declaration.
    current.priority_player = Some(PlayerId(1));
    let (action2, current2) = run_ai_decision(&current, PlayerId(1), &mut player, &reg);
    current = current2;

    if let Action::DeclareBlockers { assignments } = &action2 {
        assert!(!assignments.is_empty(), "AI should block with the creature");
        let blocks_rats = assignments.iter().any(|(blocker, attacker)| *blocker == creature && *attacker == rats);
        assert!(blocks_rats, "AI should block Typhoid Rats with the enchanted creature");
        current = engine::submit_action(&current, &action2, &reg);
    } else if matches!(&action2, Action::PassPriority) {
        // AI passed — manually set up the block and proceed.
        current = engine::submit_action(&current, &Action::DeclareBlockers {
            assignments: vec![(creature, rats)],
        }, &reg);
    } else {
        panic!("Expected DeclareBlockers or PassPriority, got {:?}", action2);
    }

    // Resolve combat damage.
    current.step = Step::CombatDamage;
    combat::deal_combat_damage(&mut current, &reg);
    check_state_based_actions_with_registry(&mut current, Some(&reg));

    // Creature should survive via regeneration.
    assert_eq!(current.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive deathtouch damage thanks to regeneration");
    assert!(current.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(current.get_object(creature).unwrap().regeneration_shields, 0,
        "Regeneration shield should be consumed");
    // Typhoid Rats should be dead (took 3 damage from the 3/3).
    assert_eq!(current.get_object(rats).unwrap().zone, Zone::Graveyard,
        "Typhoid Rats should die from combat damage");
    eprintln!("OK: AI activated regenerate → creature survived deathtouch, Rats died");
}

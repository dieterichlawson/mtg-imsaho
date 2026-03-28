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
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
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

    // P0 (AI): 2/2 creature on battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(bears).unwrap().summoning_sick = false;
    state.get_object_mut(bears).unwrap().colors = vec![Color::Green];
    state.get_object_mut(bears).unwrap().controller = PlayerId(0);

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
    // Verify outcome: creature should have a +1/+1 counter.
    assert_eq!(final_state.get_counter_count(bears, CounterType::PlusOnePlusOne), 1,
        "Travel Preparations should add a +1/+1 counter");
    assert_eq!(final_state.effective_power(bears, &reg), Some(3));
    eprintln!("OK: AI cast Travel Preparations — creature is now 3/3 with +1/+1 counter");
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
// Scenario 8: Cast Unburial Rites to reanimate a 5/5
//
// P0 (AI), a 5/5 creature in graveyard, Unburial Rites in hand,
// 5 Swamps. Cost is {4}{B}. Should cast to return the 5/5.
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

    // P0 (AI): Kindercatch (6/6) in graveyard as reanimate target
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, PlayerId(0), Zone::Graveyard, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];

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
    // Verify outcome: Kindercatch should be on the battlefield now.
    assert_eq!(final_state.get_object(kc).unwrap().zone, Zone::Battlefield,
        "Unburial Rites should return Kindercatch to the battlefield");
    eprintln!("OK: AI cast Unburial Rites — Kindercatch 6/6 reanimated");
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
// Cost is {2}{U}. Should cast for card selection (draw 1, mill 3).
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
    save_scenario(&state, "ai_forbidden_alchemy");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_forbidden_alchemy.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Forbidden Alchemy for card selection, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Forbidden Alchemy");
    // Verify outcome: drew 1 card (hand had 1, now has 1 after cast), library shrunk by 4 (1 draw + 3 mill).
    let lib_size = final_state.get_player(PlayerId(0)).library_order.len();
    assert!(lib_size <= 11, "Should have drawn+milled 4 cards, library = {}", lib_size);
    eprintln!("OK: AI cast Forbidden Alchemy — library now {}", lib_size);
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

    // P0: big 5/5 creature threatening lethal
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let big = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(5), Some(5));
    state.get_object_mut(big).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().colors = vec![Color::Green];

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
        "AI should cast Feeling of Dread to tap the 5/5, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Feeling of Dread");
    // Verify outcome: opponent's 5/5 should be tapped.
    assert!(final_state.get_object(big).unwrap().tapped,
        "Feeling of Dread should tap the target creature");
    eprintln!("OK: AI cast Feeling of Dread — 5/5 is now tapped");
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
    // Verify outcome: opponent's blocker should be tapped.
    assert!(final_state.get_object(blocker).unwrap().tapped,
        "Nightbird's Clutches should tap the target creature");
    eprintln!("OK: AI cast Nightbird's Clutches — blocker is now tapped");
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

//! AI scenario tests for card mechanics.
//!
//! Each test builds a game state where the correct play is obvious,
//! then verifies the AI makes the right decision and the mechanic
//! works correctly after resolution.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_mechanics
//! Requires ANTHROPIC_API_KEY to be set.

use std::fs;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::{AwaitingAction, GameState};
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
// Scenario: Somberwald Spider with morbid
//
// P0 (AI) has Somberwald Spider in hand + 5 Forests. A creature
// died this turn (morbid active). After casting, the spider should
// enter the battlefield with two +1/+1 counters.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_mechanics_somberwald_spider_morbid() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.creature_died_this_turn = true; // enable morbid

    // P0 (AI): Somberwald Spider in hand
    let spider_id = reg.get_id_by_name("Somberwald Spider").unwrap();
    let spider = state.create_object(spider_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(spider).unwrap().name = "Somberwald Spider".into();

    // P0 (AI): 5 Forests for {4}{G} cost
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..5 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_somberwald_spider");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_somberwald_spider.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Somberwald Spider, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Somberwald Spider");

    // Verify the spider is on the battlefield
    let spiders: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Somberwald Spider")
        .collect();
    assert_eq!(spiders.len(), 1,
        "Expected Somberwald Spider on battlefield, found {}", spiders.len());

    // Verify morbid ETB: spider should have two +1/+1 counters
    let spider_obj = spiders[0];
    let counter_count = final_state.get_counter_count(spider_obj.id, CounterType::PlusOnePlusOne);
    assert_eq!(counter_count, 2,
        "Somberwald Spider should have 2 +1/+1 counters from morbid ETB, got {}", counter_count);
    eprintln!("OK: AI cast Somberwald Spider — morbid ETB gave 2 +1/+1 counters");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Invisible Stalker attacks for lethal
//
// P0 (AI) has Invisible Stalker (1/1, can't be blocked) on the
// battlefield. Opponent at 1 life. AI should attack with the
// Stalker since it can't be blocked.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_mechanics_invisible_stalker_attack() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 1; // 1 damage is lethal
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Invisible Stalker on battlefield
    let stalker_card_id = reg.get_id_by_name("Invisible Stalker").unwrap();
    let stalker = state.create_object(stalker_card_id, PlayerId(0), Zone::Battlefield, Some(1), Some(1));
    {
        let obj = state.get_object_mut(stalker).unwrap();
        obj.name = "Invisible Stalker".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Blue];
    }

    // P1: a big blocker that would normally deter attacks
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let blocker = state.create_object(tusker_id, PlayerId(1), Zone::Battlefield, Some(5), Some(5));
    {
        let obj = state.get_object_mut(blocker).unwrap();
        obj.name = "Kalonian Tusker".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Green];
    }

    // P0 (AI): some tapped lands
    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Island".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }

    // Libraries
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let land_id = if p == 0 { island_id } else { forest_id };
        let name = if p == 0 { "Island" } else { "Forest" };
        let mut lib = Vec::new();
        for _ in 0..15 {
            let id = state.create_object(land_id, PlayerId(p), Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = name.into();
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    save_scenario(&state, "ai_invisible_stalker");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_invisible_stalker.log");
    let legal = engine::legal_actions(&state, &reg);
    assert!(legal.combat_prompt.is_some(), "Should have a combat prompt");

    let view = GameView::for_player(&state, PlayerId(0), &reg);
    let action = player.choose_combat(&view, legal.combat_prompt.as_ref().unwrap());

    match &action {
        Action::DeclareAttackers { attackers } => {
            let stalker_attacks = attackers.iter().any(|(id, _)| *id == stalker);
            assert!(stalker_attacks,
                "AI should attack with Invisible Stalker (can't be blocked, opponent at 1 life). \
                 Attackers declared: {:?}", attackers);

            // Submit the action and verify combat state
            let new_state = engine::submit_action(&state, &action, &reg);
            let combat_state = new_state.combat.as_ref()
                .expect("Combat state should exist after declaring attackers");
            assert!(combat_state.attackers.contains_key(&stalker),
                "Stalker should be in the attackers map after submitting DeclareAttackers");
            eprintln!("OK: AI attacked with Invisible Stalker ({} total attackers)", attackers.len());
        }
        other => panic!("Expected DeclareAttackers, got: {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Claustrophobia locks down a threat
//
// P0 (AI) has Claustrophobia in hand + 3 Islands. Opponent has a
// threatening 5/5 creature. AI should cast Claustrophobia on it.
// After resolve, the creature should be tapped.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_mechanics_claustrophobia() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 8; // under threat
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 5/5 creature
    let big_id = reg.get_id_by_name("Kindercatch").unwrap();
    let big = state.create_object(big_id, PlayerId(1), Zone::Battlefield, Some(5), Some(5));
    state.get_object_mut(big).unwrap().name = "Kindercatch".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().colors = vec![Color::Green];

    // P0 (AI): Claustrophobia in hand
    let claustro_id = reg.get_id_by_name("Claustrophobia").unwrap();
    let claustro = state.create_object(claustro_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(claustro).unwrap().name = "Claustrophobia".into();

    // P0 (AI): 3 Islands for {1}{U}{U} cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_claustrophobia");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_claustrophobia.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Claustrophobia on the 5/5, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Claustrophobia");

    // Verify outcome: the 5/5 should be tapped (Claustrophobia ETB taps it)
    assert!(final_state.get_object(big).unwrap().tapped,
        "Claustrophobia should tap the enchanted creature on ETB");
    eprintln!("OK: AI cast Claustrophobia — creature is tapped");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Bonds of Faith locks down a non-Human
//
// P0 (AI) has Bonds of Faith in hand + 2 Plains. Opponent has a
// threatening 3/3 non-Human creature. AI should cast Bonds of Faith
// on it. After resolve, the creature can't attack or block.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_mechanics_bonds_of_faith() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 8; // under threat
    state.players[1].life = 20;
    state.turn_number = 4;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 3/3 non-Human creature (Kalonian Tusker is a Beast)
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0 (AI): Bonds of Faith in hand
    let bonds_id = reg.get_id_by_name("Bonds of Faith").unwrap();
    let bonds = state.create_object(bonds_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(bonds).unwrap().name = "Bonds of Faith".into();

    // P0 (AI): 2 Plains for {1}{W} cost
    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..2 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_bonds_of_faith");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_bonds_of_faith.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Bonds of Faith on the non-Human creature, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bonds of Faith");

    // Verify outcome: creature can't attack (Bonds of Faith on a non-Human)
    assert!(!final_state.can_attack(threat, &reg),
        "Non-Human creature enchanted with Bonds of Faith should not be able to attack");
    let eligible = combat::eligible_attackers_with_registry(&final_state, PlayerId(1), &reg);
    assert!(!eligible.contains(&threat),
        "Enchanted creature should not appear in eligible attackers");
    eprintln!("OK: AI cast Bonds of Faith — non-Human creature can't attack or block");
}

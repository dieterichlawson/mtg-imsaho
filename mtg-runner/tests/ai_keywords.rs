//! AI scenario tests for keyword mechanics.
//!
//! Each test builds a specific game state, saves it to /tmp for manual testing,
//! then runs the Claude AI against it and verifies the correct decision.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_
//! Requires ANTHROPIC_API_KEY to be set.

use std::fs;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
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

// ═══════════════════════════════════════════════════════════════════
// Scenario 1: Flying Attack
//
// The AI (P0) has Abbey Griffin (2/2 flying) and a ground 2/2.
// The opponent (P1) has two 3/3 ground creatures.
// The Griffin can fly over the blockers — AI should attack with it.
// The ground 2/2 would die to a 3/3, so a smart AI might hold it back.
// Test: AI attacks with at least the Griffin.
// ═══════════════════════════════════════════════════════════════════

fn build_flying_attack_scenario(registry: &CardRegistry) -> GameState {
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6; // low life — attacking with flyer is very valuable
    state.turn_number = 7;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.players[0].land_plays_remaining = 0;

    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let plains_id = registry.get_id_by_name("Plains").unwrap();
    let griffin_id = registry.get_id_by_name("Abbey Griffin").unwrap();
    let bears_id = registry.get_id_by_name("Grizzly Bears").unwrap();

    // P0 (AI) battlefield: Abbey Griffin 2/2 flying + Grizzly Bears 2/2
    let griffin = state.create_object(griffin_id, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    {
        let obj = state.get_object_mut(griffin).unwrap();
        obj.name = "Abbey Griffin".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::White];
    }
    let bears = state.create_object(bears_id, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
    {
        let obj = state.get_object_mut(bears).unwrap();
        obj.name = "Grizzly Bears".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Green];
    }

    // P0 lands (tapped, already used this turn)
    for _ in 0..3 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Plains".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }
    for _ in 0..2 {
        let id = state.create_object(mountain_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Mountain".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }

    // P1 battlefield: two 3/3 ground creatures (Kalonian Tuskers)
    let tusker_id = registry.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..2 {
        let id = state.create_object(tusker_id, PlayerId(1), Zone::Battlefield, Some(3), Some(3));
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Kalonian Tusker".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Green];
    }

    // P1 lands
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    for _ in 0..4 {
        let id = state.create_object(forest_id, PlayerId(1), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Forest".into();
        obj.summoning_sick = false;
    }

    // Libraries
    for p in 0..2u8 {
        let land_id = if p == 0 { plains_id } else { forest_id };
        let mut lib = Vec::new();
        for _ in 0..15 {
            let id = state.create_object(land_id, PlayerId(p), Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = if p == 0 { "Plains" } else { "Forest" }.into();
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    state.log(mtg_engine::state::LogLevel::Milestone, "── Turn 7 (p0) ──".into());
    state
}

#[test]
#[ignore]
fn ai_flying_attack() {
    let registry = CardRegistry::with_all_cards();
    let state = build_flying_attack_scenario(&registry);
    save_scenario(&state, "flying_attack_scenario");

    let griffin_id = state.objects.values()
        .find(|o| o.name == "Abbey Griffin" && o.zone == Zone::Battlefield)
        .unwrap().id;

    let mut player = LlmPlayer::new("Attacker")
        .with_log("/tmp/flying_attack_test.log");

    let legal = engine::legal_actions(&state, &registry);
    assert!(legal.combat_prompt.is_some(), "Should have a combat prompt");

    let view = GameView::for_player(&state, PlayerId(0), &registry);
    let action = player.choose_combat(&view, legal.combat_prompt.as_ref().unwrap());

    match &action {
        Action::DeclareAttackers { attackers } => {
            let griffin_attacks = attackers.iter().any(|(id, _)| *id == griffin_id);
            assert!(griffin_attacks,
                "AI should attack with Abbey Griffin (flying, can't be blocked by ground creatures). \
                 Attackers declared: {:?}", attackers);
            eprintln!("OK: AI attacked with Abbey Griffin ({} total attackers)", attackers.len());
        }
        other => panic!("Expected DeclareAttackers, got: {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 2: Deathtouch Block
//
// The opponent (P0) attacks with Kalonian Tusker (3/3).
// The AI (P1) has Typhoid Rats (1/1 deathtouch) at 8 life.
// Blocking trades the Rats for the Tusker (deathtouch kills anything).
// Not blocking means taking 3 damage (8 → 5 life).
// Test: AI blocks with Typhoid Rats.
// ═══════════════════════════════════════════════════════════════════

fn build_deathtouch_block_scenario(registry: &CardRegistry) -> GameState {
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 8;
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;

    let tusker_id = registry.get_id_by_name("Kalonian Tusker").unwrap();
    let rats_id = registry.get_id_by_name("Typhoid Rats").unwrap();
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let swamp_id = registry.get_id_by_name("Swamp").unwrap();

    // P0 attacker: Kalonian Tusker 3/3
    let tusker = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    {
        let obj = state.get_object_mut(tusker).unwrap();
        obj.name = "Kalonian Tusker".into();
        obj.summoning_sick = false;
        obj.tapped = true; // tapped from attacking
        obj.colors = vec![Color::Green];
    }

    // Set up combat: tusker is attacking P1
    let mut combat = CombatState::new();
    combat.attackers.insert(tusker, PlayerId(1));
    combat.blocker_assignments.insert(tusker, Vec::new());
    state.combat = Some(combat);
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers {
        defending_player: PlayerId(1),
    });
    state.priority_player = Some(PlayerId(1));

    // P0 lands
    for _ in 0..4 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Forest".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }

    // P1 (AI): Typhoid Rats 1/1 deathtouch
    let rats = state.create_object(rats_id, PlayerId(1), Zone::Battlefield, Some(1), Some(1));
    {
        let obj = state.get_object_mut(rats).unwrap();
        obj.name = "Typhoid Rats".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Black];
    }

    // P1 lands
    for _ in 0..3 {
        let id = state.create_object(swamp_id, PlayerId(1), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Swamp".into();
        obj.summoning_sick = false;
    }

    // Libraries
    for p in 0..2u8 {
        let land_id = if p == 0 { forest_id } else { swamp_id };
        let mut lib = Vec::new();
        for _ in 0..15 {
            let id = state.create_object(land_id, PlayerId(p), Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = if p == 0 { "Forest" } else { "Swamp" }.into();
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    state.log(mtg_engine::state::LogLevel::Milestone, "── Turn 6 (p0) ──".into());
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Kalonian Tusker".into());
    state
}

#[test]
#[ignore]
fn ai_deathtouch_block() {
    let registry = CardRegistry::with_all_cards();
    let state = build_deathtouch_block_scenario(&registry);
    save_scenario(&state, "deathtouch_block_scenario");

    let rats_id = state.objects.values()
        .find(|o| o.name == "Typhoid Rats" && o.zone == Zone::Battlefield)
        .unwrap().id;
    let tusker_id = state.objects.values()
        .find(|o| o.name == "Kalonian Tusker" && o.zone == Zone::Battlefield)
        .unwrap().id;

    let mut player = LlmPlayer::new("Blocker")
        .with_log("/tmp/deathtouch_block_test.log");

    let legal = engine::legal_actions(&state, &registry);
    assert!(legal.combat_prompt.is_some(), "Should have a combat prompt for blockers");

    let view = GameView::for_player(&state, PlayerId(1), &registry);
    let action = player.choose_combat(&view, legal.combat_prompt.as_ref().unwrap());

    match &action {
        Action::DeclareBlockers { assignments } => {
            let rats_blocks_tusker = assignments.iter()
                .any(|(blocker, attacker)| *blocker == rats_id && *attacker == tusker_id);
            assert!(rats_blocks_tusker,
                "AI should block Kalonian Tusker with Typhoid Rats (deathtouch trades 1/1 for 3/3). \
                 Assignments: {:?}", assignments);
            eprintln!("OK: AI blocked Kalonian Tusker with Typhoid Rats (deathtouch)");
        }
        other => panic!("Expected DeclareBlockers, got: {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 3: Lifelink Attack
//
// The AI (P0) is at 4 life with Markov Patrician (3/1 lifelink).
// The opponent (P1) has no untapped creatures (can't block).
// Attacking gains 3 life (4 → 7) and deals 3 damage.
// This is an obvious attack — lifelink stabilizes the AI's life total.
// Test: AI attacks with Markov Patrician.
// ═══════════════════════════════════════════════════════════════════

fn build_lifelink_attack_scenario(registry: &CardRegistry) -> GameState {
    let mut state = GameState::new(2);
    state.players[0].life = 4; // critically low
    state.players[1].life = 15;
    state.turn_number = 8;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.players[0].land_plays_remaining = 0;

    let patrician_id = registry.get_id_by_name("Markov Patrician").unwrap();
    let swamp_id = registry.get_id_by_name("Swamp").unwrap();
    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let bears_id = registry.get_id_by_name("Grizzly Bears").unwrap();

    // P0 (AI): Markov Patrician 3/1 lifelink
    let patrician = state.create_object(patrician_id, PlayerId(0), Zone::Battlefield, Some(3), Some(1));
    {
        let obj = state.get_object_mut(patrician).unwrap();
        obj.name = "Markov Patrician".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Black];
    }

    // P0 lands
    for _ in 0..4 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Swamp".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }

    // P1: two creatures but both tapped (attacked last turn, or used abilities)
    for _ in 0..2 {
        let id = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Grizzly Bears".into();
        obj.summoning_sick = false;
        obj.tapped = true; // can't block
        obj.colors = vec![Color::Green];
    }

    // P1 lands
    for _ in 0..4 {
        let id = state.create_object(mountain_id, PlayerId(1), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Mountain".into();
        obj.summoning_sick = false;
    }

    // Libraries
    for p in 0..2u8 {
        let land_id = if p == 0 { swamp_id } else { mountain_id };
        let mut lib = Vec::new();
        for _ in 0..15 {
            let id = state.create_object(land_id, PlayerId(p), Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = if p == 0 { "Swamp" } else { "Mountain" }.into();
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    state.log(mtg_engine::state::LogLevel::Milestone, "── Turn 8 (p0) ──".into());
    state
}

#[test]
#[ignore]
fn ai_lifelink_attack() {
    let registry = CardRegistry::with_all_cards();
    let state = build_lifelink_attack_scenario(&registry);
    save_scenario(&state, "lifelink_attack_scenario");

    let patrician_id = state.objects.values()
        .find(|o| o.name == "Markov Patrician" && o.zone == Zone::Battlefield)
        .unwrap().id;

    let mut player = LlmPlayer::new("Attacker")
        .with_log("/tmp/lifelink_attack_test.log");

    let legal = engine::legal_actions(&state, &registry);
    assert!(legal.combat_prompt.is_some(), "Should have a combat prompt");

    let view = GameView::for_player(&state, PlayerId(0), &registry);
    let action = player.choose_combat(&view, legal.combat_prompt.as_ref().unwrap());

    match &action {
        Action::DeclareAttackers { attackers } => {
            let patrician_attacks = attackers.iter().any(|(id, _)| *id == patrician_id);
            assert!(patrician_attacks,
                "AI at 4 life should attack with Markov Patrician (3/1 lifelink) to gain 3 life. \
                 Attackers declared: {:?}", attackers);
            eprintln!("OK: AI attacked with Markov Patrician (lifelink, {} total attackers)", attackers.len());
        }
        other => panic!("Expected DeclareAttackers, got: {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 4: Flash Ambush Viper
//
// P0 attacks with a Goblin Piker (2/1).
// P1 (AI) has priority during DeclareBlockers with no creatures on board,
// but has Ambush Viper in hand and 2 untapped Forests.
// AI should cast Ambush Viper (flash, deathtouch) to use as a blocker.
// Test: AI taps forests and casts Ambush Viper.
// ═══════════════════════════════════════════════════════════════════

fn build_flash_ambush_scenario(registry: &CardRegistry) -> GameState {
    let mut state = GameState::new(2);
    state.players[0].life = 18;
    state.players[1].life = 6; // low life, taking 2 damage hurts
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;

    let piker_id = registry.get_id_by_name("Goblin Piker").unwrap();
    let viper_id = registry.get_id_by_name("Ambush Viper").unwrap();
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let mountain_id = registry.get_id_by_name("Mountain").unwrap();

    // P0: Goblin Piker 2/1 is attacking
    let piker = state.create_object(piker_id, PlayerId(0), Zone::Battlefield, Some(2), Some(1));
    {
        let obj = state.get_object_mut(piker).unwrap();
        obj.name = "Goblin Piker".into();
        obj.summoning_sick = false;
        obj.tapped = true;
        obj.colors = vec![Color::Red];
    }

    // Set up combat state
    let mut combat = CombatState::new();
    combat.attackers.insert(piker, PlayerId(1));
    combat.blocker_assignments.insert(piker, Vec::new());
    state.combat = Some(combat);

    // P1 has priority during declare blockers — can cast instants/flash before declaring
    // Since the engine gives priority to P1 before the blocker prompt,
    // we set this up as a priority moment (not awaiting blockers yet).
    state.priority_player = Some(PlayerId(1));
    // No awaiting_action — P1 has priority to cast spells first.

    // P0 lands
    for _ in 0..3 {
        let id = state.create_object(mountain_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Mountain".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }

    // P1: Ambush Viper in hand, 2 untapped Forests
    let viper = state.create_object(viper_id, PlayerId(1), Zone::Hand, Some(2), Some(1));
    {
        let obj = state.get_object_mut(viper).unwrap();
        obj.name = "Ambush Viper".into();
        obj.colors = vec![Color::Green];
    }
    for _ in 0..2 {
        let id = state.create_object(forest_id, PlayerId(1), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Forest".into();
        obj.summoning_sick = false;
    }

    // Libraries
    for p in 0..2u8 {
        let land_id = if p == 0 { mountain_id } else { forest_id };
        let mut lib = Vec::new();
        for _ in 0..15 {
            let id = state.create_object(land_id, PlayerId(p), Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = if p == 0 { "Mountain" } else { "Forest" }.into();
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    state.log(mtg_engine::state::LogLevel::Milestone, "── Turn 6 (p0) ──".into());
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Goblin Piker".into());
    state
}

#[test]
#[ignore]
fn ai_flash_ambush_viper() {
    let registry = CardRegistry::with_all_cards();
    let state = build_flash_ambush_scenario(&registry);
    save_scenario(&state, "flash_ambush_scenario");

    let mut player = LlmPlayer::new("Ambusher")
        .with_log("/tmp/flash_ambush_test.log");

    let mut current_state = state;

    // AI should tap forests and cast Ambush Viper (flash).
    for i in 0..10 {
        let legal = engine::legal_actions(&current_state, &registry);

        // If we get a combat prompt, the Viper was already cast and resolved.
        if legal.combat_prompt.is_some() {
            eprintln!("OK: Reached blocker declaration (Ambush Viper was cast)");
            return; // success — the Viper is on the battlefield for blocking
        }

        let view = GameView::for_player(&current_state, PlayerId(1), &registry);
        let action = player.choose_action(&view, &legal.actions);

        match &action {
            Action::CastSpell { object_id, .. } => {
                let obj = current_state.get_object(*object_id).unwrap();
                assert_eq!(obj.name, "Ambush Viper",
                    "Expected Ambush Viper cast but got: {}", obj.name);
                eprintln!("OK: AI cast Ambush Viper with flash (action #{})", i + 1);
                return; // success
            }
            Action::PassPriority => {
                panic!("AI passed priority instead of casting Ambush Viper! \
                       At 6 life, taking 2 damage from Goblin Piker is costly. \
                       AI has Ambush Viper (flash, deathtouch) in hand and mana to cast it.");
            }
            Action::ActivateManaAbility { object_id, .. } => {
                let name = current_state.get_object(*object_id)
                    .map(|o| o.name.as_str()).unwrap_or("?");
                eprintln!("  Action {}: Tapped {} for mana", i + 1, name);
                current_state = engine::submit_action(&current_state, &action, &registry);
            }
            other => {
                panic!("Unexpected action: {:?}", other);
            }
        }
    }

    panic!("AI did not cast Ambush Viper within 10 actions");
}

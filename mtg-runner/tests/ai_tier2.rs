//! AI scenario tests for Tier 2 cards.
//!
//! Each test builds a game state where the correct play is obvious,
//! then verifies Claude makes the right decision.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_tier2
//! Requires ANTHROPIC_API_KEY to be set.

use std::fs;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::{CombatState, GameState};
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

/// Run the AI decision loop. Returns the first CastSpell action the AI takes.
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
        let action = player.choose_action(&view, &legal.actions);
        match &action {
            Action::CastSpell { .. } => {
                eprintln!("  AI cast spell on action #{}", i + 1);
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
// Scenario: Bounce with Silent Departure
//
// P0 has a big 5/5 creature. P1 (AI) at 6 life has Silent Departure
// in hand and one untapped Island. Should bounce the 5/5 to save
// themselves from lethal next attack.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_silent_departure_bounces_threat() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 6;
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

    // P1 (AI): Silent Departure in hand + 1 Island
    let sd_id = reg.get_id_by_name("Silent Departure").unwrap();
    let sd = state.create_object(sd_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(sd).unwrap().name = "Silent Departure".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    let isl = state.create_object(island_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(isl).unwrap().name = "Island".into();
    state.get_object_mut(isl).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_silent_departure");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_silent_departure.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Silent Departure, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Silent Departure");
    eprintln!("OK: AI cast Silent Departure to bounce the 5/5");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Naturalize removes Pacifism from own creature
//
// P0 (AI) has a 3/3 creature locked down by opponent's Pacifism.
// Has Naturalize in hand + mana. Should destroy the Pacifism to
// free the creature for attacking.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_naturalize_frees_creature() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): 3/3 creature with opponent's Pacifism attached
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let creature = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(creature).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    let pac_id = reg.get_id_by_name("Pacifism").unwrap();
    let pac = state.create_object(pac_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(pac).unwrap().name = "Pacifism".into();
    state.get_object_mut(pac).unwrap().attached_to = Some(creature);
    state.get_object_mut(pac).unwrap().summoning_sick = false;

    // P0 hand: Naturalize
    let nat_id = reg.get_id_by_name("Naturalize").unwrap();
    let nat = state.create_object(nat_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(nat).unwrap().name = "Naturalize".into();

    // P0 lands: 2 untapped Forests
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..2 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_naturalize_pacifism");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_naturalize_pacifism.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Naturalize to remove Pacifism, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Naturalize");
    eprintln!("OK: AI cast Naturalize to free its creature from Pacifism");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Prey Upon to kill a creature
//
// P0 (AI) has a 3/3 and the opponent has a 2/2. AI has Prey Upon
// in hand + mana. Should fight to kill the 2/2 (3/3 survives with
// 2 damage).
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_prey_upon_fights() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 4;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): 3/3 creature
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let mine = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(mine).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(mine).unwrap().summoning_sick = false;
    state.get_object_mut(mine).unwrap().colors = vec![Color::Green];
    state.get_object_mut(mine).unwrap().controller = PlayerId(0);

    // P1: 2/2 creature
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let theirs = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(theirs).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(theirs).unwrap().summoning_sick = false;
    state.get_object_mut(theirs).unwrap().colors = vec![Color::Green];
    state.get_object_mut(theirs).unwrap().controller = PlayerId(1);

    // P0 hand: Prey Upon
    let pu_id = reg.get_id_by_name("Prey Upon").unwrap();
    let pu = state.create_object(pu_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(pu).unwrap().name = "Prey Upon".into();

    // P0 lands: 1 untapped Forest
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let f = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(f).unwrap().name = "Forest".into();
    state.get_object_mut(f).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_prey_upon");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_prey_upon.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Prey Upon, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Prey Upon");
    eprintln!("OK: AI cast Prey Upon to fight opponent's creature");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Smite the Monstrous kills an attacking 6/6
//
// P0 attacks with Kindercatch (6/6). P1 (AI) at 7 life has Smite
// the Monstrous in hand and priority after attackers are declared.
// Taking 6 damage is lethal. Must kill it now.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_smite_the_monstrous() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 7; // 6 damage is lethal
    state.turn_number = 7;
    state.active_player = PlayerId(0);
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;

    // P0: Kindercatch 6/6 is attacking
    let big_id = reg.get_id_by_name("Kindercatch").unwrap();
    let big = state.create_object(big_id, PlayerId(0), Zone::Battlefield, Some(6), Some(6));
    state.get_object_mut(big).unwrap().name = "Kindercatch".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().tapped = true; // tapped from attacking

    let mut combat = CombatState::new();
    combat.attackers.insert(big, PlayerId(1));
    combat.blocker_assignments.insert(big, Vec::new());
    state.combat = Some(combat);

    // P1 (AI) has priority after attackers declared
    state.priority_player = Some(PlayerId(1));

    // P1 (AI): Smite the Monstrous in hand + 4 Plains
    let smite_id = reg.get_id_by_name("Smite the Monstrous").unwrap();
    let smite = state.create_object(smite_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(smite).unwrap().name = "Smite the Monstrous".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..4 {
        let id = state.create_object(plains_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Kindercatch".into());
    save_scenario(&state, "ai_smite");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_smite.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Smite the Monstrous to survive, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Smite the Monstrous");
    eprintln!("OK: AI cast Smite the Monstrous to kill the attacking 6/6");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Victim of Night as removal
//
// P1 (AI) has a Victim of Night and 2 Swamps. P0 has a 3/3 creature
// (non-Vampire/Werewolf/Zombie). AI should kill it.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_victim_of_night() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 12;
    state.turn_number = 5;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: threatening 3/3
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P1 (AI): Victim of Night + 2 Swamps
    let von_id = reg.get_id_by_name("Victim of Night").unwrap();
    let von = state.create_object(von_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(von).unwrap().name = "Victim of Night".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    for _ in 0..2 {
        let id = state.create_object(swamp_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_victim_of_night");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_victim_of_night.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Victim of Night, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Victim of Night");
    eprintln!("OK: AI cast Victim of Night to kill the 3/3");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Geistflame finishes off low-life opponent
//
// P1 (AI) at 15 life. P0 at 1 life. AI has Geistflame in hand and
// a Mountain. Should bolt the opponent for the win.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_geistflame_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 1;
    state.players[1].life = 15;
    state.turn_number = 10;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P1 (AI): Geistflame + 1 Mountain
    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let gf = state.create_object(gf_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(gf).unwrap().name = "Geistflame".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mtn = state.create_object(mtn_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(mtn).unwrap().name = "Mountain".into();
    state.get_object_mut(mtn).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_geistflame_lethal");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_geistflame_lethal.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Geistflame for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Geistflame");
    // Verify it targets the opponent (P0), not a creature.
    if let Action::CastSpell { targets, .. } = &action {
        assert!(targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == PlayerId(0))),
            "Should target opponent for lethal damage");
    }
    eprintln!("OK: AI cast Geistflame at opponent for lethal");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Dissipate counters a threatening spell
//
// P0 casts Kindercatch (6/6). P1 (AI) has Dissipate in hand and 3
// untapped Islands. At 8 life, letting a 6/6 resolve is very bad.
// Should counter it.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_dissipate_counters() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 8;
    state.turn_number = 7;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    // Kindercatch on the stack (cast by P0)
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, PlayerId(0), Zone::Stack, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];
    state.stack.push(kc);

    // P1 (AI): Dissipate + 3 Islands
    let diss_id = reg.get_id_by_name("Dissipate").unwrap();
    let diss = state.create_object(diss_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(diss).unwrap().name = "Dissipate".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kindercatch".into());
    save_scenario(&state, "ai_dissipate");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_dissipate.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Dissipate, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Dissipate");
    eprintln!("OK: AI cast Dissipate to counter the 6/6 Kindercatch");
}

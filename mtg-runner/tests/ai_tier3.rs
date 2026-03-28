//! AI scenario tests for Tier 3 Innistrad cards.
//!
//! Each test builds a game state where the correct play is obvious,
//! then verifies Claude makes the right decision.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_tier3
//! Requires ANTHROPIC_API_KEY to be set.
//!
//! Note: These Tier 3 cards (Midnight Haunting, Moan of the Unhallowed,
//! Doomed Traveler, Village Bell-Ringer, Pitchburn Devils, Falkenrath Noble,
//! Fiend Hunter) don't need explicit card knowledge in the LLM system prompt
//! because their names and effects are visible in the action list the AI sees
//! (e.g., "Cast Midnight Haunting", "Cast Fiend Hunter").

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
// Scenario: Midnight Haunting creates blockers
//
// P0 (AI) at 8 life with no creatures. P1 has a 2/2 that will attack
// next turn. AI has Midnight Haunting + 3 Plains. Should cast it to
// create two 1/1 flying Spirit tokens as blockers.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_midnight_haunting() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 8;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 2/2 creature
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let threat = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(threat).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0 (AI): Midnight Haunting in hand + 3 Plains
    let mh_id = reg.get_id_by_name("Midnight Haunting").unwrap();
    let mh = state.create_object(mh_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(mh).unwrap().name = "Midnight Haunting".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..3 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_midnight_haunting");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_midnight_haunting.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Midnight Haunting to create blockers, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Midnight Haunting");
    eprintln!("OK: AI cast Midnight Haunting to create two 1/1 Spirit tokens");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Moan of the Unhallowed creates blockers
//
// P0 (AI) at 10 life with no creatures. P1 has a 3/3 creature.
// AI has Moan of the Unhallowed + 4 Swamps. Should cast it to
// create two 2/2 Zombie tokens to block/trade.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_moan_of_the_unhallowed() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 20;
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 3/3 creature
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, PlayerId(1), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0 (AI): Moan of the Unhallowed in hand + 4 Swamps
    let moan_id = reg.get_id_by_name("Moan of the Unhallowed").unwrap();
    let moan = state.create_object(moan_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(moan).unwrap().name = "Moan of the Unhallowed".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    for _ in 0..4 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_moan_of_the_unhallowed");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_moan_of_the_unhallowed.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Moan of the Unhallowed for blockers, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Moan of the Unhallowed");
    eprintln!("OK: AI cast Moan of the Unhallowed to create two 2/2 Zombies");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Doomed Traveler — cheap creature for 1 mana
//
// P0 (AI) has Doomed Traveler in hand + 1 Plains. It's main phase.
// Casting a 1/1 for {W} is always correct when you have mana and
// nothing else to do. Simple: AI casts the creature.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_doomed_traveler() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 2;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Doomed Traveler in hand + 1 Plains
    let dt_id = reg.get_id_by_name("Doomed Traveler").unwrap();
    let dt = state.create_object(dt_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(dt).unwrap().name = "Doomed Traveler".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let p = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(p).unwrap().name = "Plains".into();
    state.get_object_mut(p).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_doomed_traveler");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_doomed_traveler.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Doomed Traveler, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Doomed Traveler");
    eprintln!("OK: AI cast Doomed Traveler (1/1 for W, dies into a Spirit)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Village Bell-Ringer flash to untap blockers
//
// P0 attacks with a 3/3 creature. P1 (AI) has Village Bell-Ringer
// in hand + 3 Plains. P1 has a tapped 2/2 creature from last turn.
// Casting VBR with flash untaps the 2/2, giving P1 two potential
// blockers (VBR 1/4 + the 2/2). Just verify the AI casts VBR.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_village_bell_ringer_flash() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 5; // low life makes blocking critical
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;

    // P0: 3/3 attacking
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let attacker = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(attacker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(attacker).unwrap().summoning_sick = false;
    state.get_object_mut(attacker).unwrap().tapped = true; // tapped from attacking
    state.get_object_mut(attacker).unwrap().colors = vec![Color::Green];

    let mut combat = CombatState::new();
    combat.attackers.insert(attacker, PlayerId(1));
    combat.blocker_assignments.insert(attacker, Vec::new());
    state.combat = Some(combat);

    // P1 (AI) has priority after attackers declared
    state.priority_player = Some(PlayerId(1));

    // P1: tapped 2/2 creature (attacked last turn, still tapped)
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let tapped_blocker = state.create_object(bears_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(tapped_blocker).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(tapped_blocker).unwrap().summoning_sick = false;
    state.get_object_mut(tapped_blocker).unwrap().tapped = true;
    state.get_object_mut(tapped_blocker).unwrap().colors = vec![Color::Green];

    // P1 (AI): Village Bell-Ringer in hand + 3 Plains
    let vbr_id = reg.get_id_by_name("Village Bell-Ringer").unwrap();
    let vbr = state.create_object(vbr_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(vbr).unwrap().name = "Village Bell-Ringer".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..3 {
        let id = state.create_object(plains_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Kalonian Tusker".into());
    save_scenario(&state, "ai_village_bell_ringer");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_village_bell_ringer.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Village Bell-Ringer with flash, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Village Bell-Ringer");
    eprintln!("OK: AI cast Village Bell-Ringer to untap creatures for blocking");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Pitchburn Devils — cast a 3/3 creature
//
// P0 (AI) has Pitchburn Devils in hand + 5 Mountains. It's main
// phase with nothing else to do. Should cast the 3/3 creature.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_pitchburn_devils() {
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

    // P0 (AI): Pitchburn Devils in hand + 5 Mountains
    let pd_id = reg.get_id_by_name("Pitchburn Devils").unwrap();
    let pd = state.create_object(pd_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(pd).unwrap().name = "Pitchburn Devils".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..5 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_pitchburn_devils");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_pitchburn_devils.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Pitchburn Devils, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Pitchburn Devils");
    eprintln!("OK: AI cast Pitchburn Devils (3/3 that deals 3 on death)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Falkenrath Noble — cast a 2/2 flyer
//
// P0 (AI) has Falkenrath Noble in hand + 4 Swamps. It's main phase
// with nothing else to do. A 2/2 flyer for 4 mana is always worth
// casting. Simple: AI casts the creature.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_falkenrath_noble() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 (AI): Falkenrath Noble in hand + 4 Swamps
    let fn_id = reg.get_id_by_name("Falkenrath Noble").unwrap();
    let noble = state.create_object(fn_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(noble).unwrap().name = "Falkenrath Noble".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    for _ in 0..4 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_falkenrath_noble");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_falkenrath_noble.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Falkenrath Noble, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Falkenrath Noble");
    eprintln!("OK: AI cast Falkenrath Noble (2/2 flyer with drain ability)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Fiend Hunter exiles a big threat
//
// P0 (AI) has Fiend Hunter in hand + 3 Plains. Opponent has a 5/5
// creature. Should cast Fiend Hunter to exile it (ETB auto-targets
// the strongest opponent creature).
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_fiend_hunter() {
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

    // P1: threatening 5/5 creature
    let big_id = reg.get_id_by_name("Kindercatch").unwrap();
    let big = state.create_object(big_id, PlayerId(1), Zone::Battlefield, Some(5), Some(5));
    state.get_object_mut(big).unwrap().name = "Kindercatch".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().colors = vec![Color::Green];

    // P0 (AI): Fiend Hunter in hand + 3 Plains
    let fh_id = reg.get_id_by_name("Fiend Hunter").unwrap();
    let fh = state.create_object(fh_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(fh).unwrap().name = "Fiend Hunter".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..3 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_fiend_hunter");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_fiend_hunter.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Fiend Hunter to exile the 5/5, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Fiend Hunter");
    eprintln!("OK: AI cast Fiend Hunter to exile the opponent's 5/5");
}

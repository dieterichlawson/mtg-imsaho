//! AI scenario tests for Tier 2 cards.
//!
//! Each test builds a game state where the correct play is obvious,
//! then verifies Claude makes the right decision.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_tier2
//! Requires ANTHROPIC_API_KEY to be set.

use std::fs;

use mtg_engine::actions::{Action, ResolvedChoice};
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
    // Verify the 5/5 was bounced to its owner's hand
    assert_eq!(final_state.get_object(big).unwrap().zone, Zone::Hand,
        "Kalonian Tusker should be bounced to hand after Silent Departure resolves");
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
    // Verify Pacifism was destroyed
    assert_eq!(final_state.get_object(pac).unwrap().zone, Zone::Graveyard,
        "Pacifism should be in graveyard after Naturalize resolves");
    eprintln!("OK: AI cast Naturalize to free its creature from Pacifism");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Prey Upon to clear blocker for lethal
//
// P0 (AI) has a 3/3, opponent at 3 life with a 2/2 blocker.
// Fighting kills the blocker so the 3/3 can attack unblocked for
// lethal. Prey Upon is the only card in hand — no other play wins.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_prey_upon_fights() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 3; // 3/3 is lethal if unblocked
    state.turn_number = 6;
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
    // Verify the opponent's 2/2 died from the fight (3 damage kills it)
    assert_eq!(final_state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "Opponent's Grizzly Bears should be dead after fight with 3/3");
    // Verify AI's 3/3 took 2 damage from the fight but survived
    let mine_obj = final_state.get_object(mine).unwrap();
    assert_eq!(mine_obj.zone, Zone::Battlefield,
        "AI's Kalonian Tusker should survive the fight");
    assert!(mine_obj.damage_marked >= 2,
        "AI's creature should have at least 2 damage marked from fight");
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
    // Verify the 6/6 was destroyed
    assert_eq!(final_state.get_object(big).unwrap().zone, Zone::Graveyard,
        "Kindercatch should be in graveyard after Smite the Monstrous resolves");
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
    // Verify the target creature was destroyed
    assert_eq!(final_state.get_object(threat).unwrap().zone, Zone::Graveyard,
        "Kalonian Tusker should be in graveyard after Victim of Night resolves");
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
    // Verify opponent is dead
    assert!(final_state.players[0].life <= 0,
        "Opponent should be at 0 or less life after Geistflame, got {}", final_state.players[0].life);
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
    // Verify the countered spell was exiled (Dissipate exiles instead of graveyard)
    assert_eq!(final_state.get_object(kc).unwrap().zone, Zone::Exile,
        "Kindercatch should be exiled after being countered by Dissipate");
    eprintln!("OK: AI cast Dissipate to counter the 6/6 Kindercatch");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Rebuke destroys an attacking creature
//
// P0 attacks with a 3/3. P1 (AI) at 4 life has Rebuke in hand and
// 3 Plains during DeclareBlockers. Taking 3 is nearly lethal with
// no blockers available. Must use Rebuke.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_rebuke_kills_attacker() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 4; // 3 damage is nearly lethal
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;

    // P0: 3/3 attacking (tapped from attack declaration)
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let attacker = state.create_object(tusker_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(attacker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(attacker).unwrap().summoning_sick = false;
    state.get_object_mut(attacker).unwrap().tapped = true;
    state.get_object_mut(attacker).unwrap().colors = vec![Color::Green];

    let mut combat = CombatState::new();
    combat.attackers.insert(attacker, PlayerId(1));
    combat.blocker_assignments.insert(attacker, Vec::new());
    state.combat = Some(combat);

    // P1 has priority during DeclareBlockers (can cast instants before declaring)
    state.priority_player = Some(PlayerId(1));

    // P1 (AI): Rebuke + 3 Plains
    let rebuke_id = reg.get_id_by_name("Rebuke").unwrap();
    let rebuke = state.create_object(rebuke_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(rebuke).unwrap().name = "Rebuke".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..3 {
        let id = state.create_object(plains_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Kalonian Tusker".into());
    save_scenario(&state, "ai_rebuke");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_rebuke.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Rebuke on the attacker, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rebuke");
    // Verify the attacking creature was destroyed
    assert_eq!(final_state.get_object(attacker).unwrap().zone, Zone::Graveyard,
        "Kalonian Tusker should be in graveyard after Rebuke resolves");
    eprintln!("OK: AI cast Rebuke to destroy the attacking 3/3");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Brimstone Volley for lethal
//
// P1 (AI) at 15 life. P0 at 3 life. AI has Brimstone Volley in hand
// and 3 Mountains. Should fire at opponent for lethal.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_brimstone_volley_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 5; // morbid deals 5, exactly lethal
    state.players[1].life = 15;
    state.turn_number = 8;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;
    state.creature_died_this_turn = true; // enable morbid

    let bv_id = reg.get_id_by_name("Brimstone Volley").unwrap();
    let bv = state.create_object(bv_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(bv).unwrap().name = "Brimstone Volley".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..3 {
        let id = state.create_object(mtn_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_brimstone_volley");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_brimstone_volley.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Brimstone Volley for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Brimstone Volley");
    if let Action::CastSpell { targets, .. } = &action {
        assert!(targets.iter().any(|t| matches!(t, mtg_engine::actions::Target::Player(p) if *p == PlayerId(0))),
            "Should target opponent for lethal");
    }
    // Verify opponent is dead (morbid Brimstone Volley deals 5)
    assert!(final_state.get_player(PlayerId(0)).life <= 0,
        "Opponent should be at 0 or less life after morbid Brimstone Volley, got {}",
        final_state.get_player(PlayerId(0)).life);
    eprintln!("OK: AI cast Brimstone Volley (morbid) at opponent for lethal — 5 damage");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Bump in the Night for lethal
//
// P1 (AI) at 10 life. P0 at 2 life. AI has Bump in the Night and
// a Swamp. 3 life loss is lethal.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_bump_in_the_night_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 2;
    state.players[1].life = 10;
    state.turn_number = 9;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    let bump_id = reg.get_id_by_name("Bump in the Night").unwrap();
    let bump = state.create_object(bump_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(bump).unwrap().name = "Bump in the Night".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let sw = state.create_object(swamp_id, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(sw).unwrap().name = "Swamp".into();
    state.get_object_mut(sw).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_bump");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_bump.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Bump in the Night for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bump in the Night");
    // Verify opponent is dead (started at 2, Bump causes 3 life loss)
    assert!(final_state.players[0].life <= 0,
        "Opponent should be at 0 or less life after Bump in the Night, got {}", final_state.players[0].life);
    eprintln!("OK: AI cast Bump in the Night for lethal");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Bramblecrush destroys opponent's Sol Ring
//
// P0 has Sol Ring producing extra mana. P1 (AI) has Bramblecrush
// and 4 Forests. Destroying Sol Ring cuts off P0's mana advantage.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_bramblecrush_destroys_artifact() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 15;
    state.turn_number = 4;
    state.active_player = PlayerId(1);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: Sol Ring on battlefield
    let ring_id = reg.get_id_by_name("Sol Ring").unwrap();
    let ring = state.create_object(ring_id, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(ring).unwrap().name = "Sol Ring".into();
    state.get_object_mut(ring).unwrap().summoning_sick = false;

    // P0 also has some lands and a creature
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..3 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    // P1 (AI): Bramblecrush + 4 Forests
    let bc_id = reg.get_id_by_name("Bramblecrush").unwrap();
    let bc = state.create_object(bc_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(bc).unwrap().name = "Bramblecrush".into();

    for _ in 0..4 {
        let id = state.create_object(forest_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_bramblecrush");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_bramblecrush.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Bramblecrush, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bramblecrush");
    // Verify Sol Ring was destroyed
    assert_eq!(final_state.get_object(ring).unwrap().zone, Zone::Graveyard,
        "Sol Ring should be in graveyard after Bramblecrush resolves");
    eprintln!("OK: AI cast Bramblecrush to destroy Sol Ring");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Urgent Exorcism destroys an opponent's Spirit
//
// P0 has a Chapel Geist (2/3 flying Spirit) that's been attacking.
// P1 (AI) at 6 life has Urgent Exorcism and 2 Plains. Must remove
// the flyer or die to aerial attacks.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_urgent_exorcism_kills_spirit() {
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

    // P0: Chapel Geist (2/3 flying Spirit)
    let geist_id = reg.get_id_by_name("Chapel Geist").unwrap();
    let geist = state.create_object(geist_id, PlayerId(0), Zone::Battlefield, Some(2), Some(3));
    state.get_object_mut(geist).unwrap().name = "Chapel Geist".into();
    state.get_object_mut(geist).unwrap().summoning_sick = false;
    state.get_object_mut(geist).unwrap().colors = vec![Color::White];

    // P1 (AI): Urgent Exorcism + 2 Plains
    let ue_id = reg.get_id_by_name("Urgent Exorcism").unwrap();
    let ue = state.create_object(ue_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(ue).unwrap().name = "Urgent Exorcism".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..2 {
        let id = state.create_object(plains_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_urgent_exorcism");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_urgent_exorcism.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Urgent Exorcism on the Spirit, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Urgent Exorcism");
    // Verify Chapel Geist was destroyed
    assert_eq!(final_state.get_object(geist).unwrap().zone, Zone::Graveyard,
        "Chapel Geist should be in graveyard after Urgent Exorcism resolves");
    eprintln!("OK: AI cast Urgent Exorcism to destroy Chapel Geist");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Frightful Delusion counters a threatening spell
//
// P0 casts Kalonian Tusker (3/3). P1 (AI) at 6 life has Frightful
// Delusion and 3 Islands. P0 has {1} in mana pool so the "pay {1}?"
// choice is not auto-resolved. Opponent declines to pay, so the
// spell is countered.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_frightful_delusion_counters() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    // Kalonian Tusker on the stack
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let tusker = state.create_object(tusker_id, PlayerId(0), Zone::Stack, Some(3), Some(3));
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(tusker).unwrap().colors = vec![Color::Green];
    state.stack.push(tusker);

    // P0: give mana in pool so "pay {1}?" is a real choice (not auto-resolved)
    state.players[0].mana_pool.add(ManaType::Colorless, 1);

    // P1 (AI): Frightful Delusion + 3 Islands
    let fd_id = reg.get_id_by_name("Frightful Delusion").unwrap();
    let fd = state.create_object(fd_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(fd).unwrap().name = "Frightful Delusion".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kalonian Tusker".into());
    save_scenario(&state, "ai_frightful_delusion");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_frightful_delusion.log");
    let (action, mut final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Frightful Delusion, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Frightful Delusion");

    // After run_ai_decision, the opponent (P0) has a "pay {1}?" choice.
    assert!(matches!(&final_state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice { player, .. }) if *player == PlayerId(0)),
        "Should have a ResolutionChoice for the opponent");
    // Opponent chooses not to pay -- spell is countered.
    final_state = engine::submit_action(&final_state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) }, &reg);
    mtg_engine::sba::check_state_based_actions_with_registry(&mut final_state, Some(&reg));

    assert_eq!(final_state.get_object(tusker).unwrap().zone, Zone::Graveyard,
        "Kalonian Tusker should be in graveyard after being countered by Frightful Delusion");
    eprintln!("OK: AI cast Frightful Delusion to counter the 3/3 (opponent declined to pay)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Lost in the Mist is the only way to survive
//
// P0 casts Kindercatch (6/6) and has a 3/3 on the battlefield.
// P1 (AI) at 3 life — the 3/3 alone is lethal next attack, and a
// 6/6 resolving makes it even worse. Lost in the Mist is the ONLY
// card in hand and counters the 6/6 + bounces the 3/3, clearing
// all threats. No other play survives.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier2_lost_in_the_mist() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 3; // 3/3 is lethal, must counter+bounce
    state.turn_number = 8;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    // P0: 3/3 on battlefield
    let bears_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let tusker = state.create_object(bears_id, PlayerId(0), Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(tusker).unwrap().summoning_sick = false;
    state.get_object_mut(tusker).unwrap().colors = vec![Color::Green];

    // Kindercatch on the stack
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, PlayerId(0), Zone::Stack, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];
    state.stack.push(kc);

    // P1 (AI): Lost in the Mist + 5 Islands
    let litm_id = reg.get_id_by_name("Lost in the Mist").unwrap();
    let litm = state.create_object(litm_id, PlayerId(1), Zone::Hand, None, None);
    state.get_object_mut(litm).unwrap().name = "Lost in the Mist".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    for _ in 0..5 {
        let id = state.create_object(island_id, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kindercatch".into());
    save_scenario(&state, "ai_lost_in_the_mist");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_lost_in_the_mist.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(1), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Lost in the Mist, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Lost in the Mist");
    // Verify the countered spell went to graveyard
    assert_eq!(final_state.get_object(kc).unwrap().zone, Zone::Graveyard,
        "Kindercatch should be in graveyard after being countered by Lost in the Mist");
    // Verify the bounced creature is back in its owner's hand
    assert_eq!(final_state.get_object(tusker).unwrap().zone, Zone::Hand,
        "Kalonian Tusker should be bounced to hand by Lost in the Mist");
    eprintln!("OK: AI cast Lost in the Mist to counter and bounce");
}

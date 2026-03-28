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

    // Verify outcome: two Spirit tokens on the battlefield
    let spirit_tokens: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.is_token && o.card_types.contains(&CardType::Creature))
        .collect();
    assert_eq!(spirit_tokens.len(), 2,
        "Expected 2 Spirit tokens on battlefield, found {}", spirit_tokens.len());
    eprintln!("OK: AI cast Midnight Haunting — 2 Spirit tokens on battlefield");
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

    // Verify outcome: two Zombie tokens on the battlefield
    let zombie_tokens: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.is_token && o.card_types.contains(&CardType::Creature))
        .collect();
    assert_eq!(zombie_tokens.len(), 2,
        "Expected 2 Zombie tokens on battlefield, found {}", zombie_tokens.len());
    eprintln!("OK: AI cast Moan of the Unhallowed — 2 Zombie tokens on battlefield");
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

    // Verify outcome: Doomed Traveler resolved onto the battlefield
    let travelers: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Doomed Traveler")
        .collect();
    assert_eq!(travelers.len(), 1,
        "Expected Doomed Traveler on battlefield, found {}", travelers.len());
    eprintln!("OK: AI cast Doomed Traveler — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Village Bell-Ringer flash to survive lethal attack
//
// P0 attacks with a 3/3. P1 (AI) at 3 life — this attack is LETHAL
// if unblocked. P1 has no untapped creatures but has VBR in hand
// with flash. Casting VBR gives P1 a 1/4 body that can block the
// 3/3. It also untaps the tapped 2/2 as a bonus. Without casting
// VBR, P1 dies. No other play survives.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_village_bell_ringer_flash() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 3; // 3/3 is LETHAL — must block or die
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

    // Verify outcome: Village Bell-Ringer on the battlefield
    let vbr_on_bf: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(1))
        .into_iter()
        .filter(|o| o.name == "Village Bell-Ringer")
        .collect();
    assert_eq!(vbr_on_bf.len(), 1,
        "Expected Village Bell-Ringer on battlefield, found {}", vbr_on_bf.len());

    // Verify ETB trigger: the previously tapped Grizzly Bears should be untapped
    let bears = final_state.get_object(tapped_blocker).unwrap();
    assert!(!bears.tapped,
        "Village Bell-Ringer ETB should have untapped Grizzly Bears");
    eprintln!("OK: AI cast Village Bell-Ringer — on battlefield, tapped creature untapped");
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

    // Verify outcome: Pitchburn Devils on the battlefield
    let devils: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Pitchburn Devils")
        .collect();
    assert_eq!(devils.len(), 1,
        "Expected Pitchburn Devils on battlefield, found {}", devils.len());
    eprintln!("OK: AI cast Pitchburn Devils — resolved on battlefield");
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

    // Verify outcome: Falkenrath Noble on the battlefield with flying
    let nobles: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Falkenrath Noble")
        .collect();
    assert_eq!(nobles.len(), 1,
        "Expected Falkenrath Noble on battlefield, found {}", nobles.len());
    assert!(final_state.has_keyword(nobles[0].id, Keyword::Flying, &reg),
        "Falkenrath Noble should have flying");
    eprintln!("OK: AI cast Falkenrath Noble — on battlefield with flying");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Fiend Hunter exiles to survive lethal
//
// P0 (AI) at 5 life, opponent has a 5/5 that will attack for lethal
// next turn. Fiend Hunter is the only card in hand — exiles the
// 5/5 on entry. Without casting, P0 dies.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_fiend_hunter() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 5; // 5/5 attack is lethal
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

    // Verify outcome: Fiend Hunter on the battlefield
    let hunters: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Fiend Hunter")
        .collect();
    assert_eq!(hunters.len(), 1,
        "Expected Fiend Hunter on battlefield, found {}", hunters.len());

    // Verify ETB trigger: Kindercatch should be exiled
    let kindercatch = final_state.get_object(big).unwrap();
    assert_eq!(kindercatch.zone, Zone::Exile,
        "Fiend Hunter ETB should have exiled Kindercatch, but it's in {:?}", kindercatch.zone);
    eprintln!("OK: AI cast Fiend Hunter — on battlefield, Kindercatch exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Mausoleum Guard — cast the creature
//
// P0 (AI) has Mausoleum Guard in hand + 4 Plains. Main phase, nothing
// else to do. Should cast the 2/2 (its dies trigger is a bonus).
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_mausoleum_guard() {
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

    let mg_id = reg.get_id_by_name("Mausoleum Guard").unwrap();
    let mg = state.create_object(mg_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(mg).unwrap().name = "Mausoleum Guard".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..4 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_mausoleum_guard");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_mausoleum_guard.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Mausoleum Guard, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Mausoleum Guard");

    // Verify outcome: Mausoleum Guard on the battlefield
    let guards: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Mausoleum Guard")
        .collect();
    assert_eq!(guards.len(), 1,
        "Expected Mausoleum Guard on battlefield, found {}", guards.len());
    eprintln!("OK: AI cast Mausoleum Guard — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Rage Thrower — cast the creature
//
// P0 (AI) has Rage Thrower + 6 Mountains. Should cast the 4/2.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_rage_thrower() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 20;
    state.turn_number = 6;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let rt_id = reg.get_id_by_name("Rage Thrower").unwrap();
    let rt = state.create_object(rt_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(rt).unwrap().name = "Rage Thrower".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    for _ in 0..6 {
        let id = state.create_object(mtn_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_rage_thrower");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_rage_thrower.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Rage Thrower, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rage Thrower");

    // Verify outcome: Rage Thrower on the battlefield
    let throwers: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Rage Thrower")
        .collect();
    assert_eq!(throwers.len(), 1,
        "Expected Rage Thrower on battlefield, found {}", throwers.len());
    eprintln!("OK: AI cast Rage Thrower — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Slayer of the Wicked — cast to destroy opponent's Zombie
//
// P0 (AI) has Slayer + 4 Plains. Opponent has Walking Corpse (Zombie).
// Should cast Slayer to trigger ETB and destroy the Zombie.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_slayer_of_the_wicked() {
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

    // Opponent has a Walking Corpse (Zombie)
    let wc_id = reg.get_id_by_name("Walking Corpse").unwrap();
    let wc = state.create_object(wc_id, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(wc).unwrap().name = "Walking Corpse".into();
    state.get_object_mut(wc).unwrap().summoning_sick = false;
    state.get_object_mut(wc).unwrap().colors = vec![Color::Black];

    let sw_id = reg.get_id_by_name("Slayer of the Wicked").unwrap();
    let sw = state.create_object(sw_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(sw).unwrap().name = "Slayer of the Wicked".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..4 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_slayer");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_slayer.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Slayer of the Wicked, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Slayer of the Wicked");

    // Verify outcome: Slayer of the Wicked on the battlefield
    let slayers: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Slayer of the Wicked")
        .collect();
    assert_eq!(slayers.len(), 1,
        "Expected Slayer of the Wicked on battlefield, found {}", slayers.len());

    // Verify ETB trigger: Walking Corpse (Zombie) should be in graveyard
    let corpse = final_state.get_object(wc).unwrap();
    assert_eq!(corpse.zone, Zone::Graveyard,
        "Slayer ETB should have destroyed Walking Corpse, but it's in {:?}", corpse.zone);
    eprintln!("OK: AI cast Slayer of the Wicked — on battlefield, Walking Corpse destroyed");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Intangible Virtue — cast the anthem
//
// P0 (AI) has two creatures and Intangible Virtue + 2 Plains.
// Casting the anthem buffs all creatures. Should cast it.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_intangible_virtue() {
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

    // P0 has two creatures
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..2 {
        let id = state.create_object(bears_id, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        state.get_object_mut(id).unwrap().colors = vec![Color::Green];
    }

    let iv_id = reg.get_id_by_name("Intangible Virtue").unwrap();
    let iv = state.create_object(iv_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(iv).unwrap().name = "Intangible Virtue".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..2 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_intangible_virtue");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_intangible_virtue.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Intangible Virtue, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Intangible Virtue");

    // Verify outcome: Intangible Virtue on the battlefield
    let virtues: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Intangible Virtue")
        .collect();
    assert_eq!(virtues.len(), 1,
        "Expected Intangible Virtue on battlefield, found {}", virtues.len());
    eprintln!("OK: AI cast Intangible Virtue — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Unruly Mob — cast the creature
//
// P0 (AI) has Unruly Mob + Plains. Turn 2, should cast the 1-drop.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_unruly_mob() {
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

    let um_id = reg.get_id_by_name("Unruly Mob").unwrap();
    let um = state.create_object(um_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(um).unwrap().name = "Unruly Mob".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..2 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_unruly_mob");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_unruly_mob.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Unruly Mob, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Unruly Mob");

    // Verify outcome: Unruly Mob on the battlefield
    let mobs: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Unruly Mob")
        .collect();
    assert_eq!(mobs.len(), 1,
        "Expected Unruly Mob on battlefield, found {}", mobs.len());
    eprintln!("OK: AI cast Unruly Mob — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Lumberknot — cast the hexproof creature
//
// P0 (AI) has Lumberknot + 4 Forests. Should cast the hexproof 1/1.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_lumberknot() {
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

    let lk_id = reg.get_id_by_name("Lumberknot").unwrap();
    let lk = state.create_object(lk_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(lk).unwrap().name = "Lumberknot".into();

    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..4 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_lumberknot");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_lumberknot.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Lumberknot, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Lumberknot");

    // Verify outcome: Lumberknot on the battlefield
    let knots: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Lumberknot")
        .collect();
    assert_eq!(knots.len(), 1,
        "Expected Lumberknot on battlefield, found {}", knots.len());
    eprintln!("OK: AI cast Lumberknot — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Elder Cathar — cast the creature
//
// P0 (AI) has Elder Cathar + 3 Plains. Should cast the 2/2.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_elder_cathar() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 3;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let ec_id = reg.get_id_by_name("Elder Cathar").unwrap();
    let ec = state.create_object(ec_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(ec).unwrap().name = "Elder Cathar".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    for _ in 0..3 {
        let id = state.create_object(plains_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_elder_cathar");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_elder_cathar.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Elder Cathar, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Elder Cathar");

    // Verify outcome: Elder Cathar on the battlefield
    let cathars: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Elder Cathar")
        .collect();
    assert_eq!(cathars.len(), 1,
        "Expected Elder Cathar on battlefield, found {}", cathars.len());
    eprintln!("OK: AI cast Elder Cathar — resolved on battlefield");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Village Cannibals — cast the creature
//
// P0 (AI) has Village Cannibals + 3 Swamps. Should cast the 2/2.
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ai_tier3_village_cannibals() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 3;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(0));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let vc_id = reg.get_id_by_name("Village Cannibals").unwrap();
    let vc = state.create_object(vc_id, PlayerId(0), Zone::Hand, None, None);
    state.get_object_mut(vc).unwrap().name = "Village Cannibals".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    for _ in 0..3 {
        let id = state.create_object(swamp_id, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    add_libraries(&mut state, &reg);
    save_scenario(&state, "ai_village_cannibals");

    let mut player = LlmPlayer::new("AI").with_log("/tmp/ai_village_cannibals.log");
    let (action, final_state) = run_ai_decision(&state, PlayerId(0), &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "AI should cast Village Cannibals, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Village Cannibals");

    // Verify outcome: Village Cannibals on the battlefield
    let cannibals: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, PlayerId(0))
        .into_iter()
        .filter(|o| o.name == "Village Cannibals")
        .collect();
    assert_eq!(cannibals.len(), 1,
        "Expected Village Cannibals on battlefield, found {}", cannibals.len());
    eprintln!("OK: AI cast Village Cannibals — resolved on battlefield");
}

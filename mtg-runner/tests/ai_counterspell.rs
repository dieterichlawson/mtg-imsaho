//! Test that Claude correctly uses Counterspell when an opponent's spell is on the stack.
//!
//! Run with: cargo test -p mtg-runner -- --ignored ai_counterspell
//! Requires ANTHROPIC_API_KEY to be set.
//!
//! The scenario: it's P0's main phase, P0 just cast Kalonian Tusker (3/3).
//! P1 (Claude) has priority with Counterspell in hand and 2 untapped Islands.
//! Claude should tap the Islands and counter the Tusker.

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

/// Build a mid-game state where P1 should counter P0's Kalonian Tusker.
///
/// Board state:
///   P0 (Red/Green): 20 life, 3 Mountains + 2 Forests (tapped), Goblin Piker 2/1
///     Hand: Lightning Bolt, Mountain
///     Just cast Kalonian Tusker — it's on the stack.
///
///   P1 (Blue/White): 14 life, 3 Islands + 1 Plains (all untapped), Savannah Lions 2/1
///     Hand: Counterspell, Coral Merfolk, Island
///     Has priority (P0 passed after casting).
fn build_counterspell_scenario(registry: &CardRegistry) -> GameState {
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 14;
    state.turn_number = 5;
    state.active_player = PlayerId(0);
    state.priority_player = Some(PlayerId(1));
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;
    state.players[0].land_plays_remaining = 0;

    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let island_id = registry.get_id_by_name("Island").unwrap();
    let plains_id = registry.get_id_by_name("Plains").unwrap();
    let goblin_piker_id = registry.get_id_by_name("Goblin Piker").unwrap();
    let kalonian_tusker_id = registry.get_id_by_name("Kalonian Tusker").unwrap();
    let counterspell_id = registry.get_id_by_name("Counterspell").unwrap();
    let savannah_lions_id = registry.get_id_by_name("Savannah Lions").unwrap();
    let coral_merfolk_id = registry.get_id_by_name("Coral Merfolk").unwrap();
    let lightning_bolt_id = registry.get_id_by_name("Lightning Bolt").unwrap();

    // --- P0 (Red/Green) battlefield ---
    for _ in 0..3 {
        let id = state.create_object(mountain_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Mountain".into();
        obj.summoning_sick = false;
    }
    for _ in 0..2 {
        let id = state.create_object(forest_id, PlayerId(0), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Forest".into();
        obj.tapped = true; // used to cast Tusker
        obj.summoning_sick = false;
    }
    {
        let id = state.create_object(goblin_piker_id, PlayerId(0), Zone::Battlefield, Some(2), Some(1));
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Goblin Piker".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Red];
    }

    // P0 hand
    {
        let id = state.create_object(lightning_bolt_id, PlayerId(0), Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = "Lightning Bolt".into();
    }
    {
        let id = state.create_object(mountain_id, PlayerId(0), Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
    }

    // P0 library
    let mut p0_lib = Vec::new();
    for _ in 0..15 {
        let id = state.create_object(mountain_id, PlayerId(0), Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        p0_lib.push(id);
    }
    state.players[0].library_order = p0_lib;

    // Kalonian Tusker on the stack (cast by P0)
    let tusker_data = registry.card_data(kalonian_tusker_id).unwrap();
    let tusker = state.create_object(
        kalonian_tusker_id, PlayerId(0), Zone::Stack,
        tusker_data.power, tusker_data.toughness,
    );
    {
        let obj = state.get_object_mut(tusker).unwrap();
        obj.name = "Kalonian Tusker".into();
        obj.colors = vec![Color::Green];
    }
    state.stack.push(tusker);

    // --- P1 (Blue/White) battlefield ---
    for _ in 0..3 {
        let id = state.create_object(island_id, PlayerId(1), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Island".into();
        obj.summoning_sick = false;
    }
    {
        let id = state.create_object(plains_id, PlayerId(1), Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Plains".into();
        obj.summoning_sick = false;
    }
    {
        let id = state.create_object(savannah_lions_id, PlayerId(1), Zone::Battlefield, Some(2), Some(1));
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Savannah Lions".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::White];
    }

    // P1 hand
    {
        let id = state.create_object(counterspell_id, PlayerId(1), Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = "Counterspell".into();
    }
    {
        let id = state.create_object(coral_merfolk_id, PlayerId(1), Zone::Hand, Some(2), Some(1));
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Coral Merfolk".into();
        obj.colors = vec![Color::Blue];
    }
    {
        let id = state.create_object(island_id, PlayerId(1), Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
    }

    // P1 library
    let mut p1_lib = Vec::new();
    for _ in 0..15 {
        let id = state.create_object(island_id, PlayerId(1), Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        p1_lib.push(id);
    }
    state.players[1].library_order = p1_lib;

    // Add some game log for context
    state.log(mtg_engine::state::LogLevel::Milestone, "── Turn 5 (p0) ──".into());
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kalonian Tusker".into());

    state
}

#[test]
#[ignore] // requires ANTHROPIC_API_KEY
fn ai_counterspell() {
    let registry = CardRegistry::with_all_cards();
    let state = build_counterspell_scenario(&registry);

    // Also write save file so it can be tested manually:
    //   cargo run -- --resume /tmp/counterspell_scenario.json --p1 random --p2 claude -q
    let save = SaveData {
        state: state.clone(),
        player_names: vec!["Red/Green".into(), "Blue/White".into()],
    };
    let json = serde_json::to_string_pretty(&save).unwrap();
    fs::write("/tmp/counterspell_scenario.json", &json).unwrap();
    eprintln!("Saved scenario to /tmp/counterspell_scenario.json");

    // Create Claude player for P1
    let mut player = LlmPlayer::new("BlueWhite")
        .with_log("/tmp/counterspell_test.log");

    let mut current_state = state;

    // Decision loop: Claude should tap Islands then cast Counterspell.
    // Mana abilities don't use the stack, so the sequence is:
    //   1. Tap Island ({U})
    //   2. Tap Island ({U}{U})
    //   3. Cast Counterspell targeting Kalonian Tusker
    for i in 0..10 {
        let legal = engine::legal_actions(&current_state, &registry);
        let view = GameView::for_player(&current_state, PlayerId(1), &registry);
        let action = player.choose_action(&view, &legal);

        match &action {
            Action::CastSpell { object_id, .. } => {
                let obj = current_state.get_object(*object_id).unwrap();
                assert_eq!(obj.name, "Counterspell",
                    "Expected Counterspell cast but got: {}", obj.name);
                eprintln!("OK: Claude cast Counterspell (action #{})", i + 1);

                // Submit the CastSpell action (puts Counterspell on the stack)
                current_state = engine::submit_action(&current_state, &action, &registry);

                // Resolve the top of stack (Counterspell resolves, countering Kalonian Tusker)
                mtg_engine::stack::resolve_top_of_stack(&mut current_state, &registry);

                // Verify Kalonian Tusker ended up in the graveyard (not battlefield or stack)
                let tusker_in_graveyard = current_state.objects_in_zone(Zone::Graveyard, PlayerId(0))
                    .iter().any(|o| o.name == "Kalonian Tusker");
                let tusker_on_battlefield = current_state.all_objects_in_zone(Zone::Battlefield)
                    .iter().any(|o| o.name == "Kalonian Tusker");
                let tusker_on_stack = current_state.all_objects_in_zone(Zone::Stack)
                    .iter().any(|o| o.name == "Kalonian Tusker");
                assert!(tusker_in_graveyard,
                    "Kalonian Tusker should be in the graveyard after being countered");
                assert!(!tusker_on_battlefield,
                    "Kalonian Tusker should NOT be on the battlefield");
                assert!(!tusker_on_stack,
                    "Kalonian Tusker should NOT be on the stack");

                // Verify Counterspell is in the graveyard
                let counterspell_in_graveyard = current_state.objects_in_zone(Zone::Graveyard, PlayerId(1))
                    .iter().any(|o| o.name == "Counterspell");
                assert!(counterspell_in_graveyard,
                    "Counterspell should be in the graveyard after resolving");

                eprintln!("OK: Kalonian Tusker countered and both spells in graveyard");
                return; // success
            }
            Action::PassPriority => {
                panic!("Claude passed priority instead of countering Kalonian Tusker!");
            }
            Action::ActivateManaAbility { object_id, .. } => {
                let name = current_state.get_object(*object_id)
                    .map(|o| o.name.as_str()).unwrap_or("?");
                eprintln!("  Action {}: Tapped {} for mana", i + 1, name);
                current_state = engine::submit_action(&current_state, &action, &registry);
            }
            Action::Concede => {
                panic!("Claude conceded instead of countering!");
            }
            other => {
                panic!("Unexpected action: {:?}", other);
            }
        }
    }

    panic!("Claude did not cast Counterspell within 10 actions");
}

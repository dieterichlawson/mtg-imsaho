//! Deterministic scenario tests using ScriptedPlayer.
//!
//! These are the non-LLM equivalents of the ai_* tests. Each test sets up
//! a specific board state, scripts the "correct" sequence of actions, and
//! verifies the game engine processes them correctly.
//!
//! Unlike the ai_* tests, these always run (no #[ignore]), require no API
//! keys, and are fully deterministic.

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;
use mtg_engine::view::GameView;

use mtg_player::scripted::ScriptedPlayer;
use mtg_player::Player;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

// ── Helpers ────────────────────────────────────────────────────────

/// Run a scripted player through a decision loop until it casts a spell
/// or runs out of actions. Handles mana abilities automatically.
/// Returns the final state and the CastSpell action that was taken.
fn run_until_cast(
    state: &GameState,
    registry: &CardRegistry,
    player_id: PlayerId,
    player: &mut ScriptedPlayer,
) -> (GameState, Action) {
    let mut current = state.clone();
    for _ in 0..20 {
        let legal = engine::legal_actions(&current, registry);
        let view = GameView::for_player(&current, player_id, registry);
        let action = player.choose_action(&view, &legal);

        match &action {
            Action::CastSpell { .. } => {
                current = engine::submit_action(&current, &action, registry);
                return (current, action);
            }
            Action::ActivateManaAbility { .. } => {
                current = engine::submit_action(&current, &action, registry);
            }
            Action::PassPriority => {
                return (current, action);
            }
            other => {
                current = engine::submit_action(&current, other, registry);
            }
        }
    }
    panic!("ScriptedPlayer did not cast a spell within 20 actions");
}

// ── Counterspell scenario ──────────────────────────────────────────

fn build_counterspell_scenario(registry: &CardRegistry) -> GameState {
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 14;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;
    state.players[0].land_plays_remaining = 0;

    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let island_id = registry.get_id_by_name("Island").unwrap();
    let plains_id = registry.get_id_by_name("Plains").unwrap();
    let kalonian_tusker_id = registry.get_id_by_name("Kalonian Tusker").unwrap();
    let counterspell_id = registry.get_id_by_name("Counterspell").unwrap();

    // P0 battlefield: 3 Mountains (tapped)
    for _ in 0..3 {
        let id = state.create_object(mountain_id, P0, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Mountain".into();
        obj.tapped = true;
        obj.summoning_sick = false;
    }

    // Kalonian Tusker on the stack
    let tusker_data = registry.card_data(kalonian_tusker_id).unwrap();
    let tusker = state.create_object(
        kalonian_tusker_id, P0, Zone::Stack,
        tusker_data.power, tusker_data.toughness,
    );
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();
    state.stack.push(tusker);

    // P1 battlefield: 3 Islands + 1 Plains (untapped)
    for _ in 0..3 {
        let id = state.create_object(island_id, P1, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Island".into();
        obj.summoning_sick = false;
    }
    {
        let id = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Plains".into();
        obj.summoning_sick = false;
    }

    // P1 hand: Counterspell
    {
        let id = state.create_object(counterspell_id, P1, Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = "Counterspell".into();
    }

    // P1 library
    let mut p1_lib = Vec::new();
    for _ in 0..10 {
        let id = state.create_object(island_id, P1, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        p1_lib.push(id);
    }
    state.players[1].library_order = p1_lib;

    // P0 library
    let mut p0_lib = Vec::new();
    for _ in 0..10 {
        let id = state.create_object(mountain_id, P0, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        p0_lib.push(id);
    }
    state.players[0].library_order = p0_lib;

    state
}

/// P1 should tap two Islands and cast Counterspell targeting Kalonian Tusker.
#[test]
fn scripted_counterspell() {
    let registry = CardRegistry::with_all_cards();
    let state = build_counterspell_scenario(&registry);

    // Find the object IDs we need for scripting.
    let islands: Vec<_> = state.objects.values()
        .filter(|o| o.name == "Island" && o.zone == Zone::Battlefield && o.controller == P1)
        .map(|o| o.id)
        .collect();
    let counterspell_obj = state.objects.values()
        .find(|o| o.name == "Counterspell" && o.zone == Zone::Hand)
        .unwrap();
    let tusker_obj = state.objects.values()
        .find(|o| o.name == "Kalonian Tusker" && o.zone == Zone::Stack)
        .unwrap();

    // Script: tap Island, tap Island, cast Counterspell targeting Tusker.
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::CastSpell {
            object_id: counterspell_obj.id,
            targets: vec![Target::Object(tusker_obj.id)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (mut final_state, action) = run_until_cast(&state, &registry, P1, &mut player);

    // Verify it was Counterspell that was cast.
    if let Action::CastSpell { object_id, .. } = &action {
        assert_eq!(final_state.get_object(*object_id).unwrap().name, "Counterspell");
    } else {
        panic!("Expected CastSpell action");
    }

    // Resolve Counterspell.
    mtg_engine::stack::resolve_top_of_stack(&mut final_state, &registry);

    // Tusker should be in graveyard (countered).
    assert!(
        final_state.objects_in_zone(Zone::Graveyard, P0)
            .iter().any(|o| o.name == "Kalonian Tusker"),
        "Kalonian Tusker should be countered and in graveyard"
    );
    assert!(
        !final_state.all_objects_in_zone(Zone::Battlefield)
            .iter().any(|o| o.name == "Kalonian Tusker"),
        "Kalonian Tusker should NOT be on the battlefield"
    );
}

// ── Lightning Bolt scenario ────────────────────────────────────────

/// P0 has a Lightning Bolt and a Mountain. Scripts tapping the Mountain
/// and casting Bolt at P1.
#[test]
fn scripted_bolt_to_face() {
    let registry = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 3;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;

    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let bolt_id = registry.get_id_by_name("Lightning Bolt").unwrap();

    let mountain = state.create_object(mountain_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(mountain).unwrap().name = "Mountain".into();
    state.get_object_mut(mountain).unwrap().summoning_sick = false;

    let bolt = state.create_object(bolt_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bolt).unwrap().name = "Lightning Bolt".into();

    // Fill libraries so we don't hit empty-library SBA.
    for _ in 0..10 {
        let id = state.create_object(mountain_id, P0, Zone::Library, None, None);
        state.players[0].library_order.push(id);
        let id = state.create_object(mountain_id, P1, Zone::Library, None, None);
        state.players[1].library_order.push(id);
    }

    let actions = vec![
        Action::ActivateManaAbility { object_id: mountain, ability_index: 0 },
        Action::CastSpell {
            object_id: bolt,
            targets: vec![Target::Player(P1)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (mut final_state, _) = run_until_cast(&state, &registry, P0, &mut player);

    // Resolve Bolt.
    mtg_engine::stack::resolve_top_of_stack(&mut final_state, &registry);

    assert_eq!(final_state.get_player(P1).life, 17,
        "P1 should take 3 damage from Lightning Bolt");
}

// ── Removal spell scenario ─────────────────────────────────────────

/// P0 scripts casting Doom Blade on P1's creature.
#[test]
fn scripted_doom_blade_kills_creature() {
    let registry = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 4;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;

    let swamp_id = registry.get_id_by_name("Swamp").unwrap();
    let doom_id = registry.get_id_by_name("Doom Blade").unwrap();

    // P0: 2 Swamps + Doom Blade in hand
    let swamp1 = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(swamp1).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp1).unwrap().summoning_sick = false;
    let swamp2 = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(swamp2).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp2).unwrap().summoning_sick = false;

    let doom = state.create_object(doom_id, P0, Zone::Hand, None, None);
    state.get_object_mut(doom).unwrap().name = "Doom Blade".into();

    // P1: a 4/4 creature
    let creature = state.create_object(
        registry.get_id_by_name("Kalonian Tusker").unwrap(),
        P1, Zone::Battlefield, Some(3), Some(3),
    );
    state.get_object_mut(creature).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // Fill libraries.
    for _ in 0..10 {
        let id = state.create_object(swamp_id, P0, Zone::Library, None, None);
        state.players[0].library_order.push(id);
        let id = state.create_object(swamp_id, P1, Zone::Library, None, None);
        state.players[1].library_order.push(id);
    }

    let actions = vec![
        Action::ActivateManaAbility { object_id: swamp1, ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamp2, ability_index: 0 },
        Action::CastSpell {
            object_id: doom,
            targets: vec![Target::Object(creature)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (mut final_state, _) = run_until_cast(&state, &registry, P0, &mut player);

    mtg_engine::stack::resolve_top_of_stack(&mut final_state, &registry);

    assert_eq!(
        final_state.get_object(creature).unwrap().zone,
        Zone::Graveyard,
        "Kalonian Tusker should be destroyed by Doom Blade"
    );
}

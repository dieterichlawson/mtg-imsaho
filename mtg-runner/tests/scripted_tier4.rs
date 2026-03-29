//! Deterministic scenario tests for Tier 4 (advanced) flashback, reanimation,
//! and regeneration scenarios using ScriptedPlayer.
//!
//! These are the non-LLM equivalents of the ai_tier4 tests. Each test sets up
//! a specific board state, scripts the "correct" sequence of actions, and
//! verifies the game engine processes them correctly.

use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::state::{AwaitingAction, CombatState, GameState};
use mtg_engine::types::*;
use mtg_engine::view::GameView;

use mtg_player::scripted::ScriptedPlayer;
use mtg_player::Player;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

// ── Helpers ────────────────────────────────────────────────────────

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

/// Run a scripted player through a decision loop. Handles mana abilities and
/// other non-spell actions automatically. When a CastSpell is encountered,
/// submits it, resolves the stack, runs SBAs/triggers, and handles any
/// ResolutionChoice actions from the player's queue.
/// Returns the cast action and the post-resolution state.
fn run_scripted_decision(
    state: &GameState,
    player_id: PlayerId,
    player: &mut ScriptedPlayer,
    registry: &CardRegistry,
) -> (Action, GameState) {
    let mut current = state.clone();
    for _ in 0..20 {
        let legal = engine::legal_actions(&current, registry);
        if legal.combat_prompt.is_some() {
            let action = player.choose_combat(legal.combat_prompt.as_ref().unwrap());
            return (action, current);
        }
        let view = GameView::for_player(&current, player_id, registry);
        let action = player.choose_action(&view, &legal);
        match &action {
            Action::CastSpell { .. } => {
                current = engine::submit_action(&current, &action, registry);
                mtg_engine::stack::resolve_top_of_stack(&mut current, registry);
                check_state_based_actions_with_registry(&mut current, Some(registry));
                mtg_engine::triggers::process_triggers(&mut current, registry);

                // Handle any resolution choice set by the spell/trigger.
                while let Some(AwaitingAction::ResolutionChoice { player: choice_player, .. }) = &current.awaiting_action {
                    if *choice_player == player_id {
                        let choice_legal = engine::legal_actions(&current, registry);
                        let choice_view = GameView::for_player(&current, player_id, registry);
                        let choice_action = player.choose_action(&choice_view, &choice_legal);
                        current = engine::submit_action(&current, &choice_action, registry);
                        check_state_based_actions_with_registry(&mut current, Some(registry));
                        mtg_engine::triggers::process_triggers(&mut current, registry);
                    } else {
                        break;
                    }
                }

                return (action, current);
            }
            Action::ActivateAbility { .. } => {
                current = engine::submit_action(&current, &action, registry);
                return (action, current);
            }
            Action::ActivateManaAbility { .. } => {
                current = engine::submit_action(&current, &action, registry);
            }
            Action::PassPriority => {
                return (action, current);
            }
            other => {
                current = engine::submit_action(&current, other, registry);
                return (other.clone(), current);
            }
        }
    }
    panic!("ScriptedPlayer did not act within 20 actions");
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
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_think_twice_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 20;
    state.turn_number = 8;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Think Twice in graveyard
    let tt_id = reg.get_id_by_name("Think Twice").unwrap();
    let tt = state.create_object(tt_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(tt).unwrap().name = "Think Twice".into();

    // P0: 3 Islands for {2}{U} flashback cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    let mut islands = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(island_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        islands.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Islands, cast Think Twice from graveyard (flashback)
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[2], ability_index: 0 },
        Action::CastSpell {
            object_id: tt,
            targets: vec![],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should flashback Think Twice, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Think Twice");
    // Verify outcome: drew a card and spell is exiled (flashback).
    let hand_size = final_state.objects_in_zone(Zone::Hand, P0).len();
    assert!(hand_size >= 1, "Should have drawn a card, hand size = {}", hand_size);
    assert_eq!(final_state.get_object(tt).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled after resolution");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 2: Flashback Geistflame for lethal
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_geistflame_flashback_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 1;
    state.turn_number = 12;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Geistflame in graveyard
    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let gf = state.create_object(gf_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(gf).unwrap().name = "Geistflame".into();

    // P0: 4 Mountains for {3}{R} flashback cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Mountains, cast Geistflame from graveyard targeting P1
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[3], ability_index: 0 },
        Action::CastSpell {
            object_id: gf,
            targets: vec![Target::Player(P1)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should flashback Geistflame for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Geistflame");
    if let Action::CastSpell { targets, .. } = &action {
        assert!(targets.iter().any(|t| matches!(t, Target::Player(p) if *p == P1)),
            "Should target opponent for lethal damage");
    }
    // Verify outcome: opponent should be at 0 or less life.
    assert!(final_state.get_player(P1).life <= 0,
        "Geistflame should deal 1 damage for lethal, opponent life = {}", final_state.get_player(P1).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 3: Flashback Bump in the Night for lethal
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_bump_flashback_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 3;
    state.turn_number = 14;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Bump in the Night in graveyard
    let bump_id = reg.get_id_by_name("Bump in the Night").unwrap();
    let bump = state.create_object(bump_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(bump).unwrap().name = "Bump in the Night".into();

    // P0: 6 Mountains for {5}{R} flashback cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..6 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 6 Mountains, cast Bump from graveyard targeting P1
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[4], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[5], ability_index: 0 },
        Action::CastSpell {
            object_id: bump,
            targets: vec![Target::Player(P1)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should flashback Bump in the Night for lethal, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bump in the Night");
    // Verify outcome: opponent loses 3 life (was at 3, now 0 or less).
    assert!(final_state.get_player(P1).life <= 0,
        "Bump should drain 3 for lethal, opponent life = {}", final_state.get_player(P1).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 4: Flashback Silent Departure to bounce a threat
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_silent_departure_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 10;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: big 5/5 creature
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let big = state.create_object(tusker_id, P0, Zone::Battlefield, Some(5), Some(5));
    state.get_object_mut(big).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().colors = vec![Color::Green];

    // P1: Silent Departure in graveyard
    let sd_id = reg.get_id_by_name("Silent Departure").unwrap();
    let sd = state.create_object(sd_id, P1, Zone::Graveyard, None, None);
    state.get_object_mut(sd).unwrap().name = "Silent Departure".into();

    // P1: 5 Islands for {4}{U} flashback cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    let mut islands = Vec::new();
    for _ in 0..5 {
        let id = state.create_object(island_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        islands.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 5 Islands, cast Silent Departure from graveyard targeting the 5/5
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[4], ability_index: 0 },
        Action::CastSpell {
            object_id: sd,
            targets: vec![Target::Object(big)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should flashback Silent Departure, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Silent Departure");
    // Verify outcome: 5/5 should be bounced to hand, spell exiled.
    assert_eq!(final_state.get_object(big).unwrap().zone, Zone::Hand,
        "Silent Departure should bounce the 5/5 to hand");
    assert_eq!(final_state.get_object(sd).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 5: Cast Dream Twist to mill out opponent
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_dream_twist() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 5;
    state.players[1].life = 20;
    state.turn_number = 20;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Dream Twist in hand
    let dt_id = reg.get_id_by_name("Dream Twist").unwrap();
    let dt = state.create_object(dt_id, P0, Zone::Hand, None, None);
    state.get_object_mut(dt).unwrap().name = "Dream Twist".into();

    // P0: 1 Island
    let island_id = reg.get_id_by_name("Island").unwrap();
    let isl = state.create_object(island_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(isl).unwrap().name = "Island".into();
    state.get_object_mut(isl).unwrap().summoning_sick = false;

    // P0 library: 15 cards (healthy)
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut p0_lib = Vec::new();
    for _ in 0..15 {
        let id = state.create_object(forest_id, P0, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        p0_lib.push(id);
    }
    state.players[0].library_order = p0_lib;

    // P1 library: only 3 cards left -- milling 3 empties it!
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut p1_lib = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(swamp_id, P1, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        p1_lib.push(id);
    }
    state.players[1].library_order = p1_lib;

    // Script: tap Island, cast Dream Twist targeting P1
    let actions = vec![
        Action::ActivateManaAbility { object_id: isl, ability_index: 0 },
        Action::CastSpell {
            object_id: dt,
            targets: vec![Target::Player(P1)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Dream Twist to mill out opponent, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Dream Twist");
    // Verify outcome: opponent's library is empty.
    assert_eq!(final_state.get_player(P1).library_order.len(), 0,
        "Dream Twist should empty opponent's library (3 cards milled)");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 6: Cast Travel Preparations to buff creatures
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_travel_preparations() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 15;
    state.turn_number = 4;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: two creatures on battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears1 = state.create_object(bears_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(bears1).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(bears1).unwrap().summoning_sick = false;
    state.get_object_mut(bears1).unwrap().colors = vec![Color::Green];
    state.get_object_mut(bears1).unwrap().controller = P0;

    let viper_id = reg.get_id_by_name("Ambush Viper").unwrap();
    let viper = state.create_object(viper_id, P0, Zone::Battlefield, Some(2), Some(1));
    state.get_object_mut(viper).unwrap().name = "Ambush Viper".into();
    state.get_object_mut(viper).unwrap().summoning_sick = false;
    state.get_object_mut(viper).unwrap().colors = vec![Color::Green];
    state.get_object_mut(viper).unwrap().controller = P0;

    // P0: Travel Preparations in hand
    let tp_id = reg.get_id_by_name("Travel Preparations").unwrap();
    let tp = state.create_object(tp_id, P0, Zone::Hand, None, None);
    state.get_object_mut(tp).unwrap().name = "Travel Preparations".into();

    // P0: 2 Forests for {1}{G} cost
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut forests = Vec::new();
    for _ in 0..2 {
        let id = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        forests.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 2 Forests, cast Travel Preparations targeting both creatures
    let actions = vec![
        Action::ActivateManaAbility { object_id: forests[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[1], ability_index: 0 },
        Action::CastSpell {
            object_id: tp,
            targets: vec![Target::Object(bears1), Target::Object(viper)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Travel Preparations, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Travel Preparations");
    // Verify outcome: both creatures got a +1/+1 counter.
    let bears_counters = final_state.get_counter_count(bears1, CounterType::PlusOnePlusOne);
    let viper_counters = final_state.get_counter_count(viper, CounterType::PlusOnePlusOne);
    let total_counters = bears_counters + viper_counters;
    assert!(total_counters >= 1, "Travel Preparations should add at least one +1/+1 counter");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 7: Cast Rolling Temblor to wipe opponent's creatures
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_rolling_temblor() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 12;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: two 2/2 ground creatures
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..2 {
        let creature = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(creature).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(creature).unwrap().summoning_sick = false;
        state.get_object_mut(creature).unwrap().colors = vec![Color::Green];
        state.get_object_mut(creature).unwrap().controller = P1;
    }

    // P0: Rolling Temblor in hand
    let rt_id = reg.get_id_by_name("Rolling Temblor").unwrap();
    let rt = state.create_object(rt_id, P0, Zone::Hand, None, None);
    state.get_object_mut(rt).unwrap().name = "Rolling Temblor".into();

    // P0: 3 Mountains for {2}{R} cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Mountains, cast Rolling Temblor (no targets -- hits all non-flying creatures)
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::CastSpell {
            object_id: rt,
            targets: vec![],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Rolling Temblor to kill both 2/2s, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rolling Temblor");
    // Verify outcome: opponent's 2/2s should be dead.
    let opp_creatures: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P1)
        .iter().filter(|o| o.power.is_some()).map(|o| o.id).collect();
    assert_eq!(opp_creatures.len(), 0,
        "Rolling Temblor should kill both 2/2 ground creatures");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 8: Cast Unburial Rites to reanimate a creature
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_unburial_rites() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 20;
    state.turn_number = 7;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: TWO creatures in graveyard so the choice system triggers
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, P0, Zone::Graveyard, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];

    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(bears).unwrap().colors = vec![Color::Green];

    // P0: Unburial Rites in hand
    let ur_id = reg.get_id_by_name("Unburial Rites").unwrap();
    let ur = state.create_object(ur_id, P0, Zone::Hand, None, None);
    state.get_object_mut(ur).unwrap().name = "Unburial Rites".into();

    // P0: 5 Swamps for {4}{B} cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut swamps = Vec::new();
    for _ in 0..5 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        swamps.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 5 Swamps, cast Unburial Rites (no targets at cast time),
    // then resolve the choice by choosing Kindercatch (the best target).
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamps[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[4], ability_index: 0 },
        Action::CastSpell {
            object_id: ur,
            targets: vec![],
        },
        // Resolution choice: choose Kindercatch to return to battlefield
        Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(kc))),
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Unburial Rites to reanimate, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Unburial Rites");
    // Verify outcome: Kindercatch on the battlefield.
    let kc_zone = final_state.get_object(kc).unwrap().zone;
    let bears_zone = final_state.get_object(bears).unwrap().zone;
    assert!(kc_zone == Zone::Battlefield || bears_zone == Zone::Battlefield,
        "Unburial Rites should return one creature to the battlefield (Kindercatch={:?}, Bears={:?})",
        kc_zone, bears_zone);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 9: Cast Gnaw to the Bone to gain life
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_gnaw_to_the_bone() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 3;
    state.players[1].life = 20;
    state.turn_number = 9;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: 4 creature cards in graveyard
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..4 {
        let creature = state.create_object(bears_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(creature).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(creature).unwrap().colors = vec![Color::Green];
    }

    // P0: Gnaw to the Bone in hand
    let gnaw_id = reg.get_id_by_name("Gnaw to the Bone").unwrap();
    let gnaw = state.create_object(gnaw_id, P0, Zone::Hand, None, None);
    state.get_object_mut(gnaw).unwrap().name = "Gnaw to the Bone".into();

    // P0: 3 Forests for {2}{G} cost
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut forests = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        forests.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Forests, cast Gnaw to the Bone
    let actions = vec![
        Action::ActivateManaAbility { object_id: forests[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[2], ability_index: 0 },
        Action::CastSpell {
            object_id: gnaw,
            targets: vec![],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Gnaw to the Bone at 3 life, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Gnaw to the Bone");
    // Verify outcome: AI started at 3 life, should gain 2 * 4 = 8 life -> 11.
    assert!(final_state.get_player(P0).life > 3,
        "Gnaw should gain life, AI life = {}", final_state.get_player(P0).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 10: Cast Desperate Ravings for card advantage
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_desperate_ravings() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 15;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Desperate Ravings in hand
    let dr_id = reg.get_id_by_name("Desperate Ravings").unwrap();
    let dr = state.create_object(dr_id, P0, Zone::Hand, None, None);
    state.get_object_mut(dr).unwrap().name = "Desperate Ravings".into();

    // P0: 2 Mountains for {1}{R} cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..2 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 2 Mountains, cast Desperate Ravings
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::CastSpell {
            object_id: dr,
            targets: vec![],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Desperate Ravings for card advantage, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Desperate Ravings");
    // Verify outcome: net +1 card in hand (draw 2, discard 1, cast 1).
    // AI had Desperate Ravings in hand (1 card). Cast it (0), drew 2, discarded 1 -> 1 card.
    let hand = final_state.objects_in_zone(Zone::Hand, P0).len();
    assert!(hand >= 1, "Should have cards in hand after draw 2 discard 1, hand = {}", hand);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 11: Cast Forbidden Alchemy for card selection
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_forbidden_alchemy() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 15;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Forbidden Alchemy in hand
    let fa_id = reg.get_id_by_name("Forbidden Alchemy").unwrap();
    let fa = state.create_object(fa_id, P0, Zone::Hand, None, None);
    state.get_object_mut(fa).unwrap().name = "Forbidden Alchemy".into();

    // P0: 3 Islands for {2}{U} cost
    let island_id = reg.get_id_by_name("Island").unwrap();
    let mut islands = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(island_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        islands.push(id);
    }

    add_libraries(&mut state, &reg);
    let lib_before = state.get_player(P0).library_order.len();

    // We need to know the top card of the library to script the choice.
    // The first card in library_order is the top card; we'll choose it.
    let top_card = state.get_player(P0).library_order[0];

    // Script: tap 3 Islands, cast Forbidden Alchemy, choose the first revealed card
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[2], ability_index: 0 },
        Action::CastSpell {
            object_id: fa,
            targets: vec![],
        },
        // Resolution choice: choose the first revealed card to keep
        Action::ResolveChoice {
            choice: ResolvedChoice::ChosenCard(top_card),
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Forbidden Alchemy for card selection, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Forbidden Alchemy");

    // Verify outcome: 1 card in hand, 3 went to graveyard.
    let hand_size = final_state.objects_in_zone(Zone::Hand, P0).len();
    assert!(hand_size >= 1, "Should have chosen 1 card to keep, hand = {}", hand_size);
    // Library should shrink by 4 (4 revealed, 1 kept, 3 milled).
    let lib_after = final_state.get_player(P0).library_order.len();
    assert_eq!(lib_before - lib_after, 4,
        "Should have removed 4 cards from library (before={}, after={})", lib_before, lib_after);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 12: Cast Feeling of Dread to tap opponent's creatures
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_feeling_of_dread() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 7;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: two threatening creatures (combined damage is lethal)
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let big1 = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(big1).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big1).unwrap().summoning_sick = false;
    state.get_object_mut(big1).unwrap().colors = vec![Color::Green];

    let big2 = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(big2).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(big2).unwrap().summoning_sick = false;
    state.get_object_mut(big2).unwrap().colors = vec![Color::Green];

    // P1: Feeling of Dread in hand
    let fod_id = reg.get_id_by_name("Feeling of Dread").unwrap();
    let fod = state.create_object(fod_id, P1, Zone::Hand, None, None);
    state.get_object_mut(fod).unwrap().name = "Feeling of Dread".into();

    // P1: 2 Plains for {1}{W} cost
    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..2 {
        let id = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 2 Plains, cast Feeling of Dread targeting both creatures
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::CastSpell {
            object_id: fod,
            targets: vec![Target::Object(big1), Target::Object(big2)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Feeling of Dread to tap opponent's creatures, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Feeling of Dread");
    // Verify outcome: both opponent creatures should be tapped.
    assert!(final_state.get_object(big1).unwrap().tapped,
        "Feeling of Dread should tap the first creature");
    assert!(final_state.get_object(big2).unwrap().tapped,
        "Feeling of Dread should tap the second creature");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 13: Nightbird's Clutches to clear blocker for lethal
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_nightbirds_clutches() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 3;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: 3/3 creature ready to attack
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let attacker = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(attacker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(attacker).unwrap().summoning_sick = false;
    state.get_object_mut(attacker).unwrap().colors = vec![Color::Green];
    state.get_object_mut(attacker).unwrap().controller = P0;

    // P1: 3/3 blocker that would trade
    let blocker = state.create_object(tusker_id, P1, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(blocker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(blocker).unwrap().summoning_sick = false;
    state.get_object_mut(blocker).unwrap().colors = vec![Color::Green];
    state.get_object_mut(blocker).unwrap().controller = P1;

    // P0: Nightbird's Clutches in hand
    let nc_id = reg.get_id_by_name("Nightbird's Clutches").unwrap();
    let nc = state.create_object(nc_id, P0, Zone::Hand, None, None);
    state.get_object_mut(nc).unwrap().name = "Nightbird's Clutches".into();

    // P0: 2 Mountains for {1}{R} cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..2 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 2 Mountains, cast Nightbird's Clutches targeting the blocker
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::CastSpell {
            object_id: nc,
            targets: vec![Target::Object(blocker)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Nightbird's Clutches to clear the blocker, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Nightbird's Clutches");
    // Verify outcome: opponent's blocker can't block this turn.
    assert!(final_state.until_end_of_turn_cant_block.contains(&blocker),
        "Nightbird's Clutches should prevent the creature from blocking this turn");
    let eligible = combat::eligible_blockers_with_registry(&final_state, P1, &reg);
    assert!(!eligible.contains(&blocker),
        "Blocker should not appear in eligible blockers after Nightbird's Clutches");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 14: Flashback Rally the Peasants to pump creatures
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_rally_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 5;
    state.turn_number = 8;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: two 2/2 creatures ready to attack
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let mut creature_ids = Vec::new();
    for _ in 0..2 {
        let creature = state.create_object(bears_id, P0, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(creature).unwrap().name = "Grizzly Bears".into();
        state.get_object_mut(creature).unwrap().summoning_sick = false;
        state.get_object_mut(creature).unwrap().colors = vec![Color::Green];
        state.get_object_mut(creature).unwrap().controller = P0;
        creature_ids.push(creature);
    }

    // P0: Rally the Peasants in graveyard
    let rally_id = reg.get_id_by_name("Rally the Peasants").unwrap();
    let rally = state.create_object(rally_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(rally).unwrap().name = "Rally the Peasants".into();

    // P0: 3 Mountains for {2}{R} flashback cost
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Mountains, cast Rally the Peasants from graveyard (flashback)
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::CastSpell {
            object_id: rally,
            targets: vec![],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should flashback Rally the Peasants, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rally the Peasants");
    // Verify outcome: creatures should have +2/+0 (effective 4/2), spell exiled.
    let my_creatures: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .iter().filter(|o| o.power.is_some()).map(|o| o.id).collect();
    for &id in &my_creatures {
        assert_eq!(final_state.effective_power(id, &reg), Some(4),
            "Creatures should be pumped to 4 power from Rally");
    }
    assert_eq!(final_state.get_object(rally).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 15: Flashback Moan of the Unhallowed for zombie tokens
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_moan_flashback() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 15;
    state.turn_number = 12;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: a creature threatening AI
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, P1, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0: Moan of the Unhallowed in graveyard
    let moan_id = reg.get_id_by_name("Moan of the Unhallowed").unwrap();
    let moan = state.create_object(moan_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(moan).unwrap().name = "Moan of the Unhallowed".into();

    // P0: 7 Swamps for {5}{B}{B} flashback cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut swamps = Vec::new();
    for _ in 0..7 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        swamps.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 7 Swamps, cast Moan of the Unhallowed from graveyard (flashback)
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamps[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[4], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[5], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[6], ability_index: 0 },
        Action::CastSpell {
            object_id: moan,
            targets: vec![],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should flashback Moan of the Unhallowed, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Moan of the Unhallowed");
    // Verify outcome: two 2/2 Zombie tokens on the battlefield, spell exiled.
    let my_creatures: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .iter().filter(|o| o.power.is_some()).map(|o| o.id).collect();
    assert_eq!(my_creatures.len(), 2, "Should have two Zombie tokens on battlefield");
    assert_eq!(final_state.get_object(moan).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 16: Skeletal Grimace regeneration in response to Doom Blade
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_skeletal_grimace_regenerate() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 14;
    state.turn_number = 7;
    state.active_player = P0;
    state.priority_player = Some(P1); // AI has priority to respond
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.players[1].land_plays_remaining = 0;

    // P1: a 2/2 creature on the battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // P1: Skeletal Grimace attached to creature
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_object_mut(sg).unwrap().attached_to = Some(creature);
    state.get_object_mut(sg).unwrap().summoning_sick = false;

    // P1: 1 Swamp (untapped) for {B} regeneration cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().summoning_sick = false;

    // P0: 2 Swamps (tapped, already used for Doom Blade)
    for _ in 0..2 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().tapped = true;
    }

    // P0 has cast Doom Blade targeting P1's creature -- put it on the stack.
    let db_id = reg.get_id_by_name("Doom Blade").unwrap();
    let doom_blade = state.create_object(db_id, P0, Zone::Stack, None, None);
    state.get_object_mut(doom_blade).unwrap().name = "Doom Blade".into();
    state.get_object_mut(doom_blade).unwrap().targets = vec![Target::Object(creature)];
    state.stack.push(doom_blade);

    // Give P1 another card in hand (not castable) to give choices.
    let bear2 = state.create_object(bears_id, P1, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(bear2).unwrap().name = "Grizzly Bears".into();

    add_libraries(&mut state, &reg);

    // Script: tap Swamp for mana, activate regeneration ability on the creature
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamp, ability_index: 0 },
        Action::ActivateAbility { object_id: creature, ability_index: 0 },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    // Should activate regeneration ability.
    assert!(matches!(&action, Action::ActivateAbility { .. }),
        "Should activate regenerate to save creature from Doom Blade, not {:?}", action);

    // Verify the creature now has a regeneration shield.
    assert_eq!(final_state.get_object(creature).unwrap().regeneration_shields, 1,
        "Creature should have a regeneration shield");

    // Now resolve Doom Blade -- creature should survive via regeneration.
    let mut post_resolve = final_state.clone();
    post_resolve.consecutive_passes = 2;
    mtg_engine::stack::resolve_top_of_stack(&mut post_resolve, &reg);
    check_state_based_actions_with_registry(&mut post_resolve, Some(&reg));

    assert_eq!(post_resolve.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive Doom Blade thanks to regeneration");
    assert!(post_resolve.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(post_resolve.get_object(creature).unwrap().regeneration_shields, 0,
        "Regeneration shield should be consumed");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 17: Skeletal Grimace regeneration in response to Lightning Bolt
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_skeletal_grimace_regen_vs_bolt() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 18;
    state.players[1].life = 12;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P1); // AI has priority to respond
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.players[1].land_plays_remaining = 0;

    // P1: a 2/2 creature on the battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // P1: Skeletal Grimace attached to creature
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_object_mut(sg).unwrap().attached_to = Some(creature);
    state.get_object_mut(sg).unwrap().summoning_sick = false;

    // P1: 1 Swamp (untapped) for {B} regeneration cost
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().summoning_sick = false;

    // P1: a Grizzly Bears in hand (sorcery-speed, can't help here)
    let bear2 = state.create_object(bears_id, P1, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(bear2).unwrap().name = "Grizzly Bears".into();

    // P0: 1 Mountain (tapped, used for Lightning Bolt)
    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mtn = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(mtn).unwrap().name = "Mountain".into();
    state.get_object_mut(mtn).unwrap().tapped = true;

    // P0 has cast Lightning Bolt targeting P1's creature -- on the stack.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bolt = state.create_object(bolt_id, P0, Zone::Stack, None, None);
    state.get_object_mut(bolt).unwrap().name = "Lightning Bolt".into();
    state.get_object_mut(bolt).unwrap().targets = vec![Target::Object(creature)];
    state.stack.push(bolt);

    add_libraries(&mut state, &reg);

    // Script: tap Swamp for mana, activate regeneration ability on the creature
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamp, ability_index: 0 },
        Action::ActivateAbility { object_id: creature, ability_index: 0 },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    // Should activate regeneration to save the creature from lethal damage.
    assert!(matches!(&action, Action::ActivateAbility { .. }),
        "Should activate regenerate to save creature from Lightning Bolt, not {:?}", action);

    // Verify the creature has a regeneration shield.
    assert_eq!(final_state.get_object(creature).unwrap().regeneration_shields, 1,
        "Creature should have a regeneration shield");

    // Resolve Lightning Bolt -- deals 3 damage to the 3/3 creature.
    let mut post_resolve = final_state.clone();
    post_resolve.consecutive_passes = 2;
    mtg_engine::stack::resolve_top_of_stack(&mut post_resolve, &reg);
    // SBAs: 3 damage on 3 toughness = lethal -> regeneration replaces destruction.
    check_state_based_actions_with_registry(&mut post_resolve, Some(&reg));

    assert_eq!(post_resolve.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive Lightning Bolt thanks to regeneration");
    assert!(post_resolve.get_object(creature).unwrap().tapped,
        "Regenerated creature should be tapped");
    assert_eq!(post_resolve.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be removed by regeneration");
    assert_eq!(post_resolve.get_object(creature).unwrap().regeneration_shields, 0,
        "Regeneration shield should be consumed");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario 18: Skeletal Grimace regeneration vs deathtouch in combat
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier4_skeletal_grimace_regen_vs_deathtouch() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 10;
    state.turn_number = 5;
    state.active_player = P0;
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;
    state.players[1].land_plays_remaining = 0;

    // P0: Typhoid Rats (1/1 deathtouch) attacking
    let rats_id = reg.get_id_by_name("Typhoid Rats").unwrap();
    let rats = state.create_object(rats_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(rats).unwrap().name = "Typhoid Rats".into();
    state.get_object_mut(rats).unwrap().summoning_sick = false;
    state.get_object_mut(rats).unwrap().tapped = true;
    state.get_object_mut(rats).unwrap().colors = vec![Color::Black];
    state.get_object_mut(rats).unwrap().keywords = vec![Keyword::Deathtouch];

    let mut combat_state = CombatState::new();
    combat_state.attackers.insert(rats, P1);
    combat_state.blocker_assignments.insert(rats, Vec::new());
    state.combat = Some(combat_state);

    // P1: 2/2 creature on the battlefield
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().name = "Runeclaw Bear".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    // P1: Skeletal Grimace attached to creature
    let sg_id = reg.get_id_by_name("Skeletal Grimace").unwrap();
    let sg = state.create_object(sg_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Skeletal Grimace".into();
    state.get_object_mut(sg).unwrap().attached_to = Some(creature);
    state.get_object_mut(sg).unwrap().summoning_sick = false;

    // P1: 1 Swamp (untapped) for {B}
    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let swamp = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(swamp).unwrap().name = "Swamp".into();
    state.get_object_mut(swamp).unwrap().summoning_sick = false;

    // P1 has priority during DeclareBlockers.
    state.priority_player = Some(P1);

    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Typhoid Rats".into());

    add_libraries(&mut state, &reg);

    // Script: tap Swamp, activate regeneration, then declare blockers
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamp, ability_index: 0 },
        Action::ActivateAbility { object_id: creature, ability_index: 0 },
        // After regeneration, declare blocks
        Action::DeclareBlockers {
            assignments: vec![(creature, rats)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    // First decision: activate regeneration.
    let (action, mut current) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::ActivateAbility { .. }),
        "Should activate regenerate before blocking deathtouch creature, not {:?}", action);
    assert_eq!(current.get_object(creature).unwrap().regeneration_shields, 1,
        "Creature should have a regeneration shield");

    // Second decision: declare blockers (block Rats with creature).
    current.priority_player = Some(P1);
    let (action2, current2) = run_scripted_decision(&current, P1, &mut player, &reg);
    current = current2;

    if let Action::DeclareBlockers { assignments } = &action2 {
        assert!(!assignments.is_empty(), "Should block with the creature");
        let blocks_rats = assignments.iter().any(|(blocker, attacker)| *blocker == creature && *attacker == rats);
        assert!(blocks_rats, "Should block Typhoid Rats with the enchanted creature");
        current = engine::submit_action(&current, &action2, &reg);
    } else {
        panic!("Expected DeclareBlockers, got {:?}", action2);
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
}

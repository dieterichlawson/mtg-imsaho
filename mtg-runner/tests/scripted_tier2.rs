//! Deterministic scenario tests for Tier 2 cards using ScriptedPlayer.
//!
//! Converted from ai_tier2.rs. Each test sets up a game state,
//! scripts the correct action sequence, and verifies the engine
//! processes it correctly. No LLM calls, no #[ignore], fully deterministic.

use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
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

/// Run a scripted player through a decision loop: tap mana, cast spell,
/// resolve the stack, run SBAs/triggers, handle resolution choices.
/// Returns the cast action and the post-resolution state.
fn run_scripted_decision(
    state: &GameState,
    player_id: PlayerId,
    player: &mut ScriptedPlayer,
    registry: &CardRegistry,
) -> (Action, GameState) {
    let mut current = state.clone();
    for _ in 0..15 {
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
                mtg_engine::sba::check_state_based_actions_with_registry(&mut current, Some(registry));
                mtg_engine::triggers::process_triggers(&mut current, registry);

                // Handle resolution choices for the casting player.
                while let Some(AwaitingAction::ResolutionChoice { player: choice_player, .. }) = &current.awaiting_action {
                    if *choice_player == player_id {
                        let choice_legal = engine::legal_actions(&current, registry);
                        let choice_view = GameView::for_player(&current, player_id, registry);
                        let choice_action = player.choose_action(&choice_view, &choice_legal);
                        current = engine::submit_action(&current, &choice_action, registry);
                        mtg_engine::sba::check_state_based_actions_with_registry(&mut current, Some(registry));
                        mtg_engine::triggers::process_triggers(&mut current, registry);
                    } else {
                        break;
                    }
                }

                return (action, current);
            }
            Action::ActivateManaAbility { .. } => {
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
    panic!("ScriptedPlayer did not act within 15 actions");
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
// P0 has a big 5/5 creature. P1 at 6 life has Silent Departure
// in hand and one untapped Island. Should bounce the 5/5.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_silent_departure_bounces_threat() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 6;
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

    // P1: Silent Departure in hand + 1 Island
    let sd_id = reg.get_id_by_name("Silent Departure").unwrap();
    let sd = state.create_object(sd_id, P1, Zone::Hand, None, None);
    state.get_object_mut(sd).unwrap().name = "Silent Departure".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    let isl = state.create_object(island_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(isl).unwrap().name = "Island".into();
    state.get_object_mut(isl).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap Island, cast Silent Departure targeting the 5/5
    let actions = vec![
        Action::ActivateManaAbility { object_id: isl, ability_index: 0 },
        Action::CastSpell {
            object_id: sd,
            targets: vec![Target::Object(big)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Silent Departure, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Silent Departure");
    assert_eq!(final_state.get_object(big).unwrap().zone, Zone::Hand,
        "Kalonian Tusker should be bounced to hand after Silent Departure resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Naturalize removes Pacifism from own creature
//
// P0 has a 3/3 creature locked down by opponent's Pacifism.
// Has Naturalize in hand + mana. Should destroy the Pacifism.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_naturalize_frees_creature() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: 3/3 creature with opponent's Pacifism attached
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let creature = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(creature).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().colors = vec![Color::Green];

    let pac_id = reg.get_id_by_name("Pacifism").unwrap();
    let pac = state.create_object(pac_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(pac).unwrap().name = "Pacifism".into();
    state.get_object_mut(pac).unwrap().attached_to = Some(creature);
    state.get_object_mut(pac).unwrap().summoning_sick = false;

    // P0 hand: Naturalize
    let nat_id = reg.get_id_by_name("Naturalize").unwrap();
    let nat = state.create_object(nat_id, P0, Zone::Hand, None, None);
    state.get_object_mut(nat).unwrap().name = "Naturalize".into();

    // P0 lands: 2 untapped Forests
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let f1 = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(f1).unwrap().name = "Forest".into();
    state.get_object_mut(f1).unwrap().summoning_sick = false;
    let f2 = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(f2).unwrap().name = "Forest".into();
    state.get_object_mut(f2).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap 2 Forests, cast Naturalize targeting Pacifism
    let actions = vec![
        Action::ActivateManaAbility { object_id: f1, ability_index: 0 },
        Action::ActivateManaAbility { object_id: f2, ability_index: 0 },
        Action::CastSpell {
            object_id: nat,
            targets: vec![Target::Object(pac)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);
    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Naturalize, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Naturalize");
    assert_eq!(final_state.get_object(pac).unwrap().zone, Zone::Graveyard,
        "Pacifism should be in graveyard after Naturalize resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Prey Upon to clear blocker for lethal
//
// P0 has a 3/3, opponent at 3 life with a 2/2 blocker.
// Fighting kills the blocker so the 3/3 can attack unblocked.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_prey_upon_fights() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 3;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: 3/3 creature
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let mine = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(mine).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(mine).unwrap().summoning_sick = false;
    state.get_object_mut(mine).unwrap().colors = vec![Color::Green];
    state.get_object_mut(mine).unwrap().controller = P0;

    // P1: 2/2 creature
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let theirs = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(theirs).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(theirs).unwrap().summoning_sick = false;
    state.get_object_mut(theirs).unwrap().colors = vec![Color::Green];
    state.get_object_mut(theirs).unwrap().controller = P1;

    // P0 hand: Prey Upon
    let pu_id = reg.get_id_by_name("Prey Upon").unwrap();
    let pu = state.create_object(pu_id, P0, Zone::Hand, None, None);
    state.get_object_mut(pu).unwrap().name = "Prey Upon".into();

    // P0 lands: 1 untapped Forest
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let f = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(f).unwrap().name = "Forest".into();
    state.get_object_mut(f).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap Forest, cast Prey Upon targeting [own 3/3, opponent's 2/2]
    let actions = vec![
        Action::ActivateManaAbility { object_id: f, ability_index: 0 },
        Action::CastSpell {
            object_id: pu,
            targets: vec![Target::Object(mine), Target::Object(theirs)],
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);
    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Prey Upon, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Prey Upon");
    assert_eq!(final_state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "Opponent's Grizzly Bears should be dead after fight with 3/3");
    let mine_obj = final_state.get_object(mine).unwrap();
    assert_eq!(mine_obj.zone, Zone::Battlefield,
        "Kalonian Tusker should survive the fight");
    assert!(mine_obj.damage_marked >= 2,
        "Kalonian Tusker should have at least 2 damage marked from fight");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Smite the Monstrous kills an attacking 6/6
//
// P0 attacks with Kindercatch (6/6). P1 at 7 life has Smite
// the Monstrous in hand and priority after attackers declared.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_smite_the_monstrous() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 7;
    state.turn_number = 7;
    state.active_player = P0;
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;

    // P0: Kindercatch 6/6 is attacking
    let big_id = reg.get_id_by_name("Kindercatch").unwrap();
    let big = state.create_object(big_id, P0, Zone::Battlefield, Some(6), Some(6));
    state.get_object_mut(big).unwrap().name = "Kindercatch".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().tapped = true;

    let mut combat = CombatState::new();
    combat.attackers.insert(big, P1);
    combat.blocker_assignments.insert(big, Vec::new());
    state.combat = Some(combat);

    state.priority_player = Some(P1);

    // P1: Smite the Monstrous in hand + 4 Plains
    let smite_id = reg.get_id_by_name("Smite the Monstrous").unwrap();
    let smite = state.create_object(smite_id, P1, Zone::Hand, None, None);
    state.get_object_mut(smite).unwrap().name = "Smite the Monstrous".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Kindercatch".into());

    // Script: tap 4 Plains, cast Smite the Monstrous targeting Kindercatch
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[3], ability_index: 0 },
        Action::CastSpell {
            object_id: smite,
            targets: vec![Target::Object(big)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Smite the Monstrous, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Smite the Monstrous");
    assert_eq!(final_state.get_object(big).unwrap().zone, Zone::Graveyard,
        "Kindercatch should be in graveyard after Smite the Monstrous resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Victim of Night as removal
//
// P1 has a Victim of Night and 2 Swamps. P0 has a 3/3 creature
// (non-Vampire/Werewolf/Zombie). Should kill it.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_victim_of_night() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 12;
    state.turn_number = 5;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: threatening 3/3
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P1: Victim of Night + 2 Swamps
    let von_id = reg.get_id_by_name("Victim of Night").unwrap();
    let von = state.create_object(von_id, P1, Zone::Hand, None, None);
    state.get_object_mut(von).unwrap().name = "Victim of Night".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let sw1 = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(sw1).unwrap().name = "Swamp".into();
    state.get_object_mut(sw1).unwrap().summoning_sick = false;
    let sw2 = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(sw2).unwrap().name = "Swamp".into();
    state.get_object_mut(sw2).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap 2 Swamps, cast Victim of Night targeting 3/3
    let actions = vec![
        Action::ActivateManaAbility { object_id: sw1, ability_index: 0 },
        Action::ActivateManaAbility { object_id: sw2, ability_index: 0 },
        Action::CastSpell {
            object_id: von,
            targets: vec![Target::Object(threat)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Victim of Night, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Victim of Night");
    assert_eq!(final_state.get_object(threat).unwrap().zone, Zone::Graveyard,
        "Kalonian Tusker should be in graveyard after Victim of Night resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Geistflame finishes off low-life opponent
//
// P1 at 15 life. P0 at 1 life. P1 has Geistflame in hand and
// a Mountain. Should fire at opponent for the win.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_geistflame_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 1;
    state.players[1].life = 15;
    state.turn_number = 10;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P1: Geistflame + 1 Mountain
    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let gf = state.create_object(gf_id, P1, Zone::Hand, None, None);
    state.get_object_mut(gf).unwrap().name = "Geistflame".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mtn = state.create_object(mtn_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(mtn).unwrap().name = "Mountain".into();
    state.get_object_mut(mtn).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap Mountain, cast Geistflame targeting P0
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtn, ability_index: 0 },
        Action::CastSpell {
            object_id: gf,
            targets: vec![Target::Player(P0)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Geistflame, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Geistflame");
    if let Action::CastSpell { targets, .. } = &action {
        assert!(targets.iter().any(|t| matches!(t, Target::Player(p) if *p == P0)),
            "Should target opponent for lethal damage");
    }
    assert!(final_state.players[0].life <= 0,
        "Opponent should be at 0 or less life after Geistflame, got {}", final_state.players[0].life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Dissipate counters a threatening spell
//
// P0 casts Kindercatch (6/6). P1 has Dissipate in hand and 3
// untapped Islands. At 8 life, letting a 6/6 resolve is very bad.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_dissipate_counters() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 8;
    state.turn_number = 7;
    state.active_player = P0;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    // Kindercatch on the stack (cast by P0)
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, P0, Zone::Stack, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];
    state.stack.push(kc);

    // P1: Dissipate + 3 Islands
    let diss_id = reg.get_id_by_name("Dissipate").unwrap();
    let diss = state.create_object(diss_id, P1, Zone::Hand, None, None);
    state.get_object_mut(diss).unwrap().name = "Dissipate".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    let mut islands = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(island_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        islands.push(id);
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kindercatch".into());

    // Script: tap 3 Islands, cast Dissipate targeting Kindercatch
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[2], ability_index: 0 },
        Action::CastSpell {
            object_id: diss,
            targets: vec![Target::Object(kc)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Dissipate, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Dissipate");
    assert_eq!(final_state.get_object(kc).unwrap().zone, Zone::Exile,
        "Kindercatch should be exiled after being countered by Dissipate");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Rebuke destroys an attacking creature
//
// P0 attacks with a 3/3. P1 at 4 life has Rebuke in hand and
// 3 Plains during DeclareBlockers.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_rebuke_kills_attacker() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 4;
    state.turn_number = 6;
    state.active_player = P0;
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;

    // P0: 3/3 attacking
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let attacker = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(attacker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(attacker).unwrap().summoning_sick = false;
    state.get_object_mut(attacker).unwrap().tapped = true;
    state.get_object_mut(attacker).unwrap().colors = vec![Color::Green];

    let mut combat = CombatState::new();
    combat.attackers.insert(attacker, P1);
    combat.blocker_assignments.insert(attacker, Vec::new());
    state.combat = Some(combat);

    state.priority_player = Some(P1);

    // P1: Rebuke + 3 Plains
    let rebuke_id = reg.get_id_by_name("Rebuke").unwrap();
    let rebuke = state.create_object(rebuke_id, P1, Zone::Hand, None, None);
    state.get_object_mut(rebuke).unwrap().name = "Rebuke".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 declared attackers: Kalonian Tusker".into());

    // Script: tap 3 Plains, cast Rebuke targeting the attacker
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::CastSpell {
            object_id: rebuke,
            targets: vec![Target::Object(attacker)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Rebuke, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rebuke");
    assert_eq!(final_state.get_object(attacker).unwrap().zone, Zone::Graveyard,
        "Kalonian Tusker should be in graveyard after Rebuke resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Brimstone Volley for lethal
//
// P1 at 15 life. P0 at 5 life. P1 has Brimstone Volley in hand
// and 3 Mountains. Morbid is active (creature died this turn).
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_brimstone_volley_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 5;
    state.players[1].life = 15;
    state.turn_number = 8;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;
    state.creature_died_this_turn = true; // enable morbid

    let bv_id = reg.get_id_by_name("Brimstone Volley").unwrap();
    let bv = state.create_object(bv_id, P1, Zone::Hand, None, None);
    state.get_object_mut(bv).unwrap().name = "Brimstone Volley".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(mtn_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Mountains, cast Brimstone Volley targeting P0
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::CastSpell {
            object_id: bv,
            targets: vec![Target::Player(P0)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Brimstone Volley, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Brimstone Volley");
    if let Action::CastSpell { targets, .. } = &action {
        assert!(targets.iter().any(|t| matches!(t, Target::Player(p) if *p == P0)),
            "Should target opponent for lethal");
    }
    assert!(final_state.get_player(P0).life <= 0,
        "Opponent should be at 0 or less life after morbid Brimstone Volley, got {}",
        final_state.get_player(P0).life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Bump in the Night for lethal
//
// P1 at 10 life. P0 at 2 life. P1 has Bump in the Night and
// a Swamp. 3 life loss is lethal.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_bump_in_the_night_lethal() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 2;
    state.players[1].life = 10;
    state.turn_number = 9;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    let bump_id = reg.get_id_by_name("Bump in the Night").unwrap();
    let bump = state.create_object(bump_id, P1, Zone::Hand, None, None);
    state.get_object_mut(bump).unwrap().name = "Bump in the Night".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let sw = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(sw).unwrap().name = "Swamp".into();
    state.get_object_mut(sw).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap Swamp, cast Bump in the Night targeting P0
    let actions = vec![
        Action::ActivateManaAbility { object_id: sw, ability_index: 0 },
        Action::CastSpell {
            object_id: bump,
            targets: vec![Target::Player(P0)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Bump in the Night, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bump in the Night");
    assert!(final_state.players[0].life <= 0,
        "Opponent should be at 0 or less life after Bump in the Night, got {}", final_state.players[0].life);
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Bramblecrush destroys opponent's Sol Ring
//
// P0 has Sol Ring producing extra mana. P1 has Bramblecrush
// and 4 Forests.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_bramblecrush_destroys_artifact() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 15;
    state.turn_number = 4;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: Sol Ring on battlefield
    let ring_id = reg.get_id_by_name("Sol Ring").unwrap();
    let ring = state.create_object(ring_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(ring).unwrap().name = "Sol Ring".into();
    state.get_object_mut(ring).unwrap().summoning_sick = false;

    // P0 also has some lands
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..3 {
        let id = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    // P1: Bramblecrush + 4 Forests
    let bc_id = reg.get_id_by_name("Bramblecrush").unwrap();
    let bc = state.create_object(bc_id, P1, Zone::Hand, None, None);
    state.get_object_mut(bc).unwrap().name = "Bramblecrush".into();

    let mut forests = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(forest_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        forests.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Forests, cast Bramblecrush targeting Sol Ring
    let actions = vec![
        Action::ActivateManaAbility { object_id: forests[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[3], ability_index: 0 },
        Action::CastSpell {
            object_id: bc,
            targets: vec![Target::Object(ring)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Bramblecrush, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Bramblecrush");
    assert_eq!(final_state.get_object(ring).unwrap().zone, Zone::Graveyard,
        "Sol Ring should be in graveyard after Bramblecrush resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Urgent Exorcism destroys an opponent's Spirit
//
// P0 has a Chapel Geist (2/3 flying Spirit) that's been attacking.
// P1 at 6 life has Urgent Exorcism and 2 Plains.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_urgent_exorcism_kills_spirit() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 6;
    state.active_player = P1;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[1].land_plays_remaining = 0;

    // P0: Chapel Geist (2/3 flying Spirit)
    let geist_id = reg.get_id_by_name("Chapel Geist").unwrap();
    let geist = state.create_object(geist_id, P0, Zone::Battlefield, Some(2), Some(3));
    state.get_object_mut(geist).unwrap().name = "Chapel Geist".into();
    state.get_object_mut(geist).unwrap().summoning_sick = false;
    state.get_object_mut(geist).unwrap().colors = vec![Color::White];

    // P1: Urgent Exorcism + 2 Plains
    let ue_id = reg.get_id_by_name("Urgent Exorcism").unwrap();
    let ue = state.create_object(ue_id, P1, Zone::Hand, None, None);
    state.get_object_mut(ue).unwrap().name = "Urgent Exorcism".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let pl1 = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(pl1).unwrap().name = "Plains".into();
    state.get_object_mut(pl1).unwrap().summoning_sick = false;
    let pl2 = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(pl2).unwrap().name = "Plains".into();
    state.get_object_mut(pl2).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap 2 Plains, cast Urgent Exorcism targeting Chapel Geist
    let actions = vec![
        Action::ActivateManaAbility { object_id: pl1, ability_index: 0 },
        Action::ActivateManaAbility { object_id: pl2, ability_index: 0 },
        Action::CastSpell {
            object_id: ue,
            targets: vec![Target::Object(geist)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Urgent Exorcism, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Urgent Exorcism");
    assert_eq!(final_state.get_object(geist).unwrap().zone, Zone::Graveyard,
        "Chapel Geist should be in graveyard after Urgent Exorcism resolves");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Frightful Delusion counters a threatening spell
//
// P0 casts Kalonian Tusker (3/3). P1 at 6 life has Frightful
// Delusion and 3 Islands. P0 has {1} in mana pool. Opponent
// declines to pay, so the spell is countered.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_frightful_delusion_counters() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 6;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    // Kalonian Tusker on the stack
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let tusker = state.create_object(tusker_id, P0, Zone::Stack, Some(3), Some(3));
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(tusker).unwrap().colors = vec![Color::Green];
    state.stack.push(tusker);

    // P0: give mana in pool so "pay {1}?" is a real choice
    state.players[0].mana_pool.add(ManaType::Colorless, 1);

    // P1: Frightful Delusion + 3 Islands
    let fd_id = reg.get_id_by_name("Frightful Delusion").unwrap();
    let fd = state.create_object(fd_id, P1, Zone::Hand, None, None);
    state.get_object_mut(fd).unwrap().name = "Frightful Delusion".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    let mut islands = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(island_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        islands.push(id);
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kalonian Tusker".into());

    // Script: tap 3 Islands, cast Frightful Delusion targeting Tusker on stack
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[2], ability_index: 0 },
        Action::CastSpell {
            object_id: fd,
            targets: vec![Target::Object(tusker)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, mut final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Frightful Delusion, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Frightful Delusion");

    // After resolution, the opponent (P0) has a "pay {1}?" choice.
    assert!(matches!(&final_state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice { player, .. }) if *player == P0),
        "Should have a ResolutionChoice for the opponent");
    // Opponent chooses not to pay -- spell is countered.
    final_state = engine::submit_action(&final_state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) }, &reg);
    mtg_engine::sba::check_state_based_actions_with_registry(&mut final_state, Some(&reg));

    assert_eq!(final_state.get_object(tusker).unwrap().zone, Zone::Graveyard,
        "Kalonian Tusker should be in graveyard after being countered by Frightful Delusion");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: Lost in the Mist counters + bounces
//
// P0 casts Kindercatch (6/6) and has a 3/3 on the battlefield.
// P1 at 3 life. Lost in the Mist counters the 6/6 and bounces
// the 3/3, clearing all threats.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_tier2_lost_in_the_mist() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 3;
    state.turn_number = 8;
    state.active_player = P0;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    // P0: 3/3 on battlefield
    let bears_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let tusker = state.create_object(bears_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(tusker).unwrap().summoning_sick = false;
    state.get_object_mut(tusker).unwrap().colors = vec![Color::Green];

    // Kindercatch on the stack
    let kc_id = reg.get_id_by_name("Kindercatch").unwrap();
    let kc = state.create_object(kc_id, P0, Zone::Stack, Some(6), Some(6));
    state.get_object_mut(kc).unwrap().name = "Kindercatch".into();
    state.get_object_mut(kc).unwrap().colors = vec![Color::Green];
    state.stack.push(kc);

    // P1: Lost in the Mist + 5 Islands
    let litm_id = reg.get_id_by_name("Lost in the Mist").unwrap();
    let litm = state.create_object(litm_id, P1, Zone::Hand, None, None);
    state.get_object_mut(litm).unwrap().name = "Lost in the Mist".into();

    let island_id = reg.get_id_by_name("Island").unwrap();
    let mut islands = Vec::new();
    for _ in 0..5 {
        let id = state.create_object(island_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Island".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        islands.push(id);
    }

    add_libraries(&mut state, &reg);
    state.log(mtg_engine::state::LogLevel::Event, "p0 cast Kindercatch".into());

    // Script: tap 5 Islands, cast Lost in the Mist targeting [Kindercatch on stack, Tusker on battlefield]
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[4], ability_index: 0 },
        Action::CastSpell {
            object_id: litm,
            targets: vec![Target::Object(kc), Target::Object(tusker)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);
    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Lost in the Mist, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Lost in the Mist");
    assert_eq!(final_state.get_object(kc).unwrap().zone, Zone::Graveyard,
        "Kindercatch should be in graveyard after being countered by Lost in the Mist");
    assert_eq!(final_state.get_object(tusker).unwrap().zone, Zone::Hand,
        "Kalonian Tusker should be bounced to hand by Lost in the Mist");
}

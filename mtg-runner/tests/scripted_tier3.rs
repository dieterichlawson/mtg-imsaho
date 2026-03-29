//! Deterministic scenario tests for Tier 3 Innistrad cards using ScriptedPlayer.
//!
//! Each test builds a game state where the correct play is obvious,
//! scripts the exact action sequence, and verifies the engine processes
//! it correctly.
//!
//! These tests cover tokens, ETB effects, death triggers, and anthem effects.
//! They require no API keys and run deterministically.

use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::{AwaitingAction, CombatState, GameState};
use mtg_engine::types::*;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
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

/// Run a scripted player through a decision loop: taps mana, casts a spell,
/// resolves it, runs SBAs and triggers. Returns (cast_action, final_state).
fn run_scripted_decision(
    state: &GameState,
    player_id: PlayerId,
    player: &mut ScriptedPlayer,
    registry: &CardRegistry,
) -> (Action, GameState) {
    let mut current = state.clone();
    for _ in 0..20 {
        let legal = engine::legal_actions(&current, registry);
        let view = GameView::for_player(&current, player_id, registry);
        let action = player.choose_action(&view, &legal);

        match &action {
            Action::CastSpell { .. } => {
                current = engine::submit_action(&current, &action, registry);
                mtg_engine::stack::resolve_top_of_stack(&mut current, registry);
                check_state_based_actions_with_registry(&mut current, Some(registry));
                triggers::process_triggers(&mut current, registry);

                // Handle any resolution choices from the spell/trigger.
                while let Some(AwaitingAction::ResolutionChoice { player: choice_player, .. }) = &current.awaiting_action {
                    if *choice_player == player_id {
                        let choice_legal = engine::legal_actions(&current, registry);
                        let choice_view = GameView::for_player(&current, player_id, registry);
                        let choice_action = player.choose_action(&choice_view, &choice_legal);
                        current = engine::submit_action(&current, &choice_action, registry);
                        check_state_based_actions_with_registry(&mut current, Some(registry));
                        triggers::process_triggers(&mut current, registry);
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
    panic!("ScriptedPlayer did not cast a spell within 20 actions");
}

fn spell_name<'a>(state: &'a GameState, action: &Action) -> &'a str {
    match action {
        Action::CastSpell { object_id, .. } => {
            state.get_object(*object_id).map(|o| o.name.as_str()).unwrap_or("?")
        }
        _ => "?",
    }
}

// =====================================================================
// Scenario: Midnight Haunting creates blockers
//
// P0 at 8 life with no creatures. P1 has a 2/2 that will attack
// next turn. P0 has Midnight Haunting + 3 Plains. Cast it to
// create two 1/1 flying Spirit tokens as blockers.
// =====================================================================

#[test]
fn scripted_tier3_midnight_haunting() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 8;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 2/2 creature
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let threat = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(threat).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0: Midnight Haunting in hand + 3 Plains
    let mh_id = reg.get_id_by_name("Midnight Haunting").unwrap();
    let mh = state.create_object(mh_id, P0, Zone::Hand, None, None);
    state.get_object_mut(mh).unwrap().name = "Midnight Haunting".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Plains, cast Midnight Haunting
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::CastSpell { object_id: mh, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Midnight Haunting, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Midnight Haunting");

    // Verify outcome: two Spirit tokens on the battlefield
    let spirit_tokens: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.is_token && o.card_types.contains(&CardType::Creature))
        .collect();
    assert_eq!(spirit_tokens.len(), 2,
        "Expected 2 Spirit tokens on battlefield, found {}", spirit_tokens.len());
}

// =====================================================================
// Scenario: Moan of the Unhallowed creates blockers
//
// P0 at 10 life with no creatures. P1 has a 3/3 creature.
// P0 has Moan of the Unhallowed + 4 Swamps. Cast it to
// create two 2/2 Zombie tokens to block/trade.
// =====================================================================

#[test]
fn scripted_tier3_moan_of_the_unhallowed() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 10;
    state.players[1].life = 20;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 3/3 creature
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let threat = state.create_object(tusker_id, P1, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(threat).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(threat).unwrap().summoning_sick = false;
    state.get_object_mut(threat).unwrap().colors = vec![Color::Green];

    // P0: Moan of the Unhallowed in hand + 4 Swamps
    let moan_id = reg.get_id_by_name("Moan of the Unhallowed").unwrap();
    let moan = state.create_object(moan_id, P0, Zone::Hand, None, None);
    state.get_object_mut(moan).unwrap().name = "Moan of the Unhallowed".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut swamps = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        swamps.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Swamps, cast Moan of the Unhallowed
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamps[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[3], ability_index: 0 },
        Action::CastSpell { object_id: moan, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Moan of the Unhallowed, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Moan of the Unhallowed");

    // Verify outcome: two Zombie tokens on the battlefield
    let zombie_tokens: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.is_token && o.card_types.contains(&CardType::Creature))
        .collect();
    assert_eq!(zombie_tokens.len(), 2,
        "Expected 2 Zombie tokens on battlefield, found {}", zombie_tokens.len());
}

// =====================================================================
// Scenario: Doomed Traveler -- cheap creature for 1 mana
//
// P0 has Doomed Traveler in hand + 1 Plains. Cast the creature.
// Then kill it to test the dies trigger creating a Spirit token.
// =====================================================================

#[test]
fn scripted_tier3_doomed_traveler() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 2;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Doomed Traveler in hand + 1 Plains
    let dt_id = reg.get_id_by_name("Doomed Traveler").unwrap();
    let dt = state.create_object(dt_id, P0, Zone::Hand, Some(1), Some(1));
    state.get_object_mut(dt).unwrap().name = "Doomed Traveler".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let p = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(p).unwrap().name = "Plains".into();
    state.get_object_mut(p).unwrap().summoning_sick = false;

    add_libraries(&mut state, &reg);

    // Script: tap Plains, cast Doomed Traveler
    let actions = vec![
        Action::ActivateManaAbility { object_id: p, ability_index: 0 },
        Action::CastSpell { object_id: dt, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Doomed Traveler, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Doomed Traveler");

    // Verify outcome: Doomed Traveler resolved onto the battlefield
    let dt_obj_id = {
        let v: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter().filter(|o| o.name == "Doomed Traveler").collect();
        assert_eq!(v.len(), 1, "Expected Doomed Traveler on battlefield");
        v[0].id
    };

    // Kill Doomed Traveler to test dies trigger
    final_state.events.clear();
    final_state.get_object_mut(dt_obj_id).unwrap().damage_marked = 1;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    let spirits: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter().filter(|o| o.is_token && o.name == "Spirit").collect();
    assert_eq!(spirits.len(), 1, "Doomed Traveler should create a Spirit token on death");
}

// =====================================================================
// Scenario: Village Bell-Ringer flash to survive lethal attack
//
// P0 attacks with a 3/3. P1 at 3 life -- this attack is LETHAL
// if unblocked. P1 has VBR in hand with flash. Casting VBR gives
// P1 a 1/4 body that can block the 3/3. It also untaps the tapped
// 2/2 as a bonus. Without casting VBR, P1 dies.
// =====================================================================

#[test]
fn scripted_tier3_village_bell_ringer_flash() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 3; // 3/3 is LETHAL -- must block or die
    state.turn_number = 6;
    state.active_player = P0;
    state.step = Step::DeclareAttackers;
    state.is_first_turn = false;

    // P0: 3/3 attacking
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let attacker = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(attacker).unwrap().name = "Kalonian Tusker".into();
    state.get_object_mut(attacker).unwrap().summoning_sick = false;
    state.get_object_mut(attacker).unwrap().tapped = true; // tapped from attacking
    state.get_object_mut(attacker).unwrap().colors = vec![Color::Green];

    let mut combat = CombatState::new();
    combat.attackers.insert(attacker, P1);
    combat.blocker_assignments.insert(attacker, Vec::new());
    state.combat = Some(combat);

    // P1 has priority after attackers declared
    state.priority_player = Some(P1);

    // P1: tapped 2/2 creature (attacked last turn, still tapped)
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let tapped_blocker = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(tapped_blocker).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(tapped_blocker).unwrap().summoning_sick = false;
    state.get_object_mut(tapped_blocker).unwrap().tapped = true;
    state.get_object_mut(tapped_blocker).unwrap().colors = vec![Color::Green];

    // P1: Village Bell-Ringer in hand + 3 Plains
    let vbr_id = reg.get_id_by_name("Village Bell-Ringer").unwrap();
    let vbr = state.create_object(vbr_id, P1, Zone::Hand, None, None);
    state.get_object_mut(vbr).unwrap().name = "Village Bell-Ringer".into();

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

    // Script: tap 3 Plains, cast Village Bell-Ringer (flash)
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::CastSpell { object_id: vbr, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (action, final_state) = run_scripted_decision(&state, P1, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Village Bell-Ringer with flash, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Village Bell-Ringer");

    // Verify outcome: Village Bell-Ringer on the battlefield
    let vbr_on_bf: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P1)
        .into_iter()
        .filter(|o| o.name == "Village Bell-Ringer")
        .collect();
    assert_eq!(vbr_on_bf.len(), 1,
        "Expected Village Bell-Ringer on battlefield, found {}", vbr_on_bf.len());

    // Verify ETB trigger: the previously tapped Grizzly Bears should be untapped
    let bears = final_state.get_object(tapped_blocker).unwrap();
    assert!(!bears.tapped,
        "Village Bell-Ringer ETB should have untapped Grizzly Bears");
}

// =====================================================================
// Scenario: Pitchburn Devils -- cast a 3/3 creature, then test the
// dies trigger with multiple targets (opponent + opponent creature).
//
// P0 has Pitchburn Devils in hand + 5 Mountains. P1 has a creature
// on the battlefield so there are 2+ damage targets when the Devils
// die. Script the choice to deal 3 damage to the opponent.
// =====================================================================

#[test]
fn scripted_tier3_pitchburn_devils() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: a creature on the battlefield so the dies trigger has 2+ targets
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let opp_creature = state.create_object(bears_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(opp_creature).unwrap().name = "Grizzly Bears".into();
    state.get_object_mut(opp_creature).unwrap().summoning_sick = false;
    state.get_object_mut(opp_creature).unwrap().colors = vec![Color::Green];

    // P0: Pitchburn Devils in hand + 5 Mountains
    let pd_id = reg.get_id_by_name("Pitchburn Devils").unwrap();
    let pd = state.create_object(pd_id, P0, Zone::Hand, Some(3), Some(3));
    state.get_object_mut(pd).unwrap().name = "Pitchburn Devils".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..5 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 5 Mountains, cast Pitchburn Devils
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[4], ability_index: 0 },
        Action::CastSpell { object_id: pd, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Pitchburn Devils, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Pitchburn Devils");

    // Verify outcome: Pitchburn Devils on the battlefield
    let pd_obj_id = {
        let v: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter().filter(|o| o.name == "Pitchburn Devils").collect();
        assert_eq!(v.len(), 1, "Expected Pitchburn Devils on battlefield");
        v[0].id
    };

    // Kill Pitchburn Devils to test dies trigger with multiple targets
    final_state.events.clear();
    final_state.get_object_mut(pd_obj_id).unwrap().damage_marked = 3;
    let opp_life_before = final_state.get_player(P1).life;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);

    // The trigger should set a ResolutionChoice for P0 with 2+ targets.
    assert!(matches!(&final_state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice { player, .. }) if *player == P0),
        "Should have a ResolutionChoice for P0");

    // Script: choose to deal 3 damage to the opponent player.
    let choice_action = Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTarget(Some(Target::Player(P1))),
    };
    final_state = engine::submit_action(&final_state, &choice_action, &reg);
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);

    // Verify the opponent lost 3 life.
    let opp_life_after = final_state.get_player(P1).life;
    assert_eq!(opp_life_before - opp_life_after, 3,
        "Pitchburn Devils should deal 3 damage to the opponent (life: {} -> {})",
        opp_life_before, opp_life_after);
}

// =====================================================================
// Scenario: Falkenrath Noble -- cast a 2/2 flyer
//
// P0 has Falkenrath Noble in hand + 4 Swamps. Cast the creature.
// Then kill a creature to verify the drain trigger.
// =====================================================================

#[test]
fn scripted_tier3_falkenrath_noble() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Falkenrath Noble in hand + 4 Swamps
    let fn_id = reg.get_id_by_name("Falkenrath Noble").unwrap();
    let noble = state.create_object(fn_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(noble).unwrap().name = "Falkenrath Noble".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut swamps = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        swamps.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Swamps, cast Falkenrath Noble
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamps[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[3], ability_index: 0 },
        Action::CastSpell { object_id: noble, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Falkenrath Noble, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Falkenrath Noble");

    // Verify outcome: Falkenrath Noble on the battlefield with flying
    let nobles: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.name == "Falkenrath Noble")
        .collect();
    assert_eq!(nobles.len(), 1,
        "Expected Falkenrath Noble on battlefield, found {}", nobles.len());
    assert!(final_state.has_keyword(nobles[0].id, Keyword::Flying, &reg),
        "Falkenrath Noble should have flying");

    // Create a creature and kill it to trigger Falkenrath Noble
    final_state.events.clear();
    let fodder = final_state.create_token("Fodder", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);
    final_state.get_object_mut(fodder).unwrap().damage_marked = 1;
    let p0_life_before = final_state.get_player(P0).life;
    let p1_life_before = final_state.get_player(P1).life;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    assert_eq!(final_state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain 1 life from opponent when creature dies");
    assert_eq!(final_state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should gain 1 life when creature dies");
}

// =====================================================================
// Scenario: Fiend Hunter exiles to survive lethal
//
// P0 at 5 life, opponent has a 5/5 that will attack for lethal
// next turn. Fiend Hunter is the only card in hand -- exiles the
// 5/5 on entry. Without casting, P0 dies.
// =====================================================================

#[test]
fn scripted_tier3_fiend_hunter() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 5; // 5/5 attack is lethal
    state.players[1].life = 20;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P1: threatening 5/5 creature
    let big_id = reg.get_id_by_name("Kindercatch").unwrap();
    let big = state.create_object(big_id, P1, Zone::Battlefield, Some(5), Some(5));
    state.get_object_mut(big).unwrap().name = "Kindercatch".into();
    state.get_object_mut(big).unwrap().summoning_sick = false;
    state.get_object_mut(big).unwrap().colors = vec![Color::Green];

    // P0: Fiend Hunter in hand + 3 Plains
    let fh_id = reg.get_id_by_name("Fiend Hunter").unwrap();
    let fh = state.create_object(fh_id, P0, Zone::Hand, None, None);
    state.get_object_mut(fh).unwrap().name = "Fiend Hunter".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Plains, cast Fiend Hunter (ETB auto-exiles strongest creature)
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::CastSpell { object_id: fh, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Fiend Hunter to exile the 5/5, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Fiend Hunter");

    // Verify outcome: Fiend Hunter on the battlefield
    let hunters: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.name == "Fiend Hunter")
        .collect();
    assert_eq!(hunters.len(), 1,
        "Expected Fiend Hunter on battlefield, found {}", hunters.len());

    // Verify ETB trigger: Kindercatch should be exiled
    let kindercatch = final_state.get_object(big).unwrap();
    assert_eq!(kindercatch.zone, Zone::Exile,
        "Fiend Hunter ETB should have exiled Kindercatch, but it's in {:?}", kindercatch.zone);
}

// =====================================================================
// Scenario: Mausoleum Guard -- cast the creature
//
// P0 has Mausoleum Guard in hand + 4 Plains. Cast the 2/2.
// Then kill it to test the dies trigger creating two Spirit tokens.
// =====================================================================

#[test]
fn scripted_tier3_mausoleum_guard() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 4;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let mg_id = reg.get_id_by_name("Mausoleum Guard").unwrap();
    let mg = state.create_object(mg_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(mg).unwrap().name = "Mausoleum Guard".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Plains, cast Mausoleum Guard
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[3], ability_index: 0 },
        Action::CastSpell { object_id: mg, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Mausoleum Guard, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Mausoleum Guard");

    // Verify outcome: Mausoleum Guard on the battlefield
    let mg_obj_id = {
        let v: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter().filter(|o| o.name == "Mausoleum Guard").collect();
        assert_eq!(v.len(), 1, "Expected Mausoleum Guard on battlefield");
        v[0].id
    };

    // Kill Mausoleum Guard to test dies trigger
    final_state.events.clear();
    final_state.get_object_mut(mg_obj_id).unwrap().damage_marked = 2;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    let spirits: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter().filter(|o| o.is_token && o.name == "Spirit").collect();
    assert_eq!(spirits.len(), 2, "Mausoleum Guard should create two Spirit tokens on death");
}

// =====================================================================
// Scenario: Rage Thrower -- cast the creature
//
// P0 has Rage Thrower + 6 Mountains. Cast the 4/2.
// Then kill a creature to verify the 2 damage trigger.
// =====================================================================

#[test]
fn scripted_tier3_rage_thrower() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 15;
    state.players[1].life = 20;
    state.turn_number = 6;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let rt_id = reg.get_id_by_name("Rage Thrower").unwrap();
    let rt = state.create_object(rt_id, P0, Zone::Hand, Some(4), Some(2));
    state.get_object_mut(rt).unwrap().name = "Rage Thrower".into();

    let mtn_id = reg.get_id_by_name("Mountain").unwrap();
    let mut mtns = Vec::new();
    for _ in 0..6 {
        let id = state.create_object(mtn_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Mountain".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        mtns.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 6 Mountains, cast Rage Thrower
    let actions = vec![
        Action::ActivateManaAbility { object_id: mtns[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[3], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[4], ability_index: 0 },
        Action::ActivateManaAbility { object_id: mtns[5], ability_index: 0 },
        Action::CastSpell { object_id: rt, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Rage Thrower, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Rage Thrower");

    // Verify outcome: Rage Thrower on the battlefield
    let throwers: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.name == "Rage Thrower")
        .collect();
    assert_eq!(throwers.len(), 1,
        "Expected Rage Thrower on battlefield, found {}", throwers.len());

    // Create a creature and kill it to trigger Rage Thrower
    final_state.events.clear();
    let fodder = final_state.create_token("Fodder", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);
    final_state.get_object_mut(fodder).unwrap().damage_marked = 1;
    let p1_life_before = final_state.get_player(P1).life;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    assert_eq!(final_state.get_player(P1).life, p1_life_before - 2,
        "Rage Thrower should deal 2 damage when creature dies");
}

// =====================================================================
// Scenario: Slayer of the Wicked -- cast to destroy opponent's Zombie
//
// P0 has Slayer + 4 Plains. Opponent has TWO Walking Corpses
// (Zombies) so the ETB trigger has 2+ targets and uses the choice
// system. Script choosing the first Walking Corpse.
// =====================================================================

#[test]
fn scripted_tier3_slayer_of_the_wicked() {
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

    // Opponent has TWO Walking Corpses (Zombies) so there are 2+ ETB targets
    let wc_id = reg.get_id_by_name("Walking Corpse").unwrap();
    let wc1 = state.create_object(wc_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(wc1).unwrap().name = "Walking Corpse".into();
    state.get_object_mut(wc1).unwrap().summoning_sick = false;
    state.get_object_mut(wc1).unwrap().colors = vec![Color::Black];

    let wc2 = state.create_object(wc_id, P1, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(wc2).unwrap().name = "Walking Corpse".into();
    state.get_object_mut(wc2).unwrap().summoning_sick = false;
    state.get_object_mut(wc2).unwrap().colors = vec![Color::Black];

    let sw_id = reg.get_id_by_name("Slayer of the Wicked").unwrap();
    let sw = state.create_object(sw_id, P0, Zone::Hand, None, None);
    state.get_object_mut(sw).unwrap().name = "Slayer of the Wicked".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Plains, cast Slayer of the Wicked, then choose wc1 as target
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[3], ability_index: 0 },
        Action::CastSpell { object_id: sw, targets: vec![] },
        // Resolution choice: destroy the first Walking Corpse
        Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(wc1))),
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Slayer of the Wicked, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Slayer of the Wicked");

    // Verify outcome: Slayer of the Wicked on the battlefield
    let slayers: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.name == "Slayer of the Wicked")
        .collect();
    assert_eq!(slayers.len(), 1,
        "Expected Slayer of the Wicked on battlefield, found {}", slayers.len());

    // Verify ETB trigger: first Walking Corpse destroyed, second still alive
    let wc1_zone = final_state.get_object(wc1).unwrap().zone;
    let wc2_zone = final_state.get_object(wc2).unwrap().zone;
    assert_eq!(wc1_zone, Zone::Graveyard,
        "First Walking Corpse should be destroyed (in graveyard), but is in {:?}", wc1_zone);
    assert_eq!(wc2_zone, Zone::Battlefield,
        "Second Walking Corpse should still be alive on battlefield, but is in {:?}", wc2_zone);
}

// =====================================================================
// Scenario: Intangible Virtue -- cast the anthem
//
// P0 has two Spirit tokens and Intangible Virtue + 2 Plains.
// Cast the anthem to buff all token creatures.
// =====================================================================

#[test]
fn scripted_tier3_intangible_virtue() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 4;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0 has two Spirit tokens (1/1 flying)
    let token1 = state.create_token("Spirit", P0, 1, 1, vec![Color::White],
        vec![CardType::Creature], vec![Keyword::Flying]);
    state.get_object_mut(token1).unwrap().summoning_sick = false;
    let token2 = state.create_token("Spirit", P0, 1, 1, vec![Color::White],
        vec![CardType::Creature], vec![Keyword::Flying]);
    state.get_object_mut(token2).unwrap().summoning_sick = false;

    let iv_id = reg.get_id_by_name("Intangible Virtue").unwrap();
    let iv = state.create_object(iv_id, P0, Zone::Hand, None, None);
    state.get_object_mut(iv).unwrap().name = "Intangible Virtue".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..2 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 2 Plains, cast Intangible Virtue
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::CastSpell { object_id: iv, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Intangible Virtue, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Intangible Virtue");

    // Verify outcome: Intangible Virtue on the battlefield
    let virtues: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
        .into_iter()
        .filter(|o| o.name == "Intangible Virtue")
        .collect();
    assert_eq!(virtues.len(), 1,
        "Expected Intangible Virtue on battlefield, found {}", virtues.len());

    // Verify tokens got +1/+1 (effective 2/2) and have vigilance
    for &tid in &[token1, token2] {
        let eff_pow = final_state.effective_power(tid, &reg).unwrap();
        let eff_tou = final_state.effective_toughness(tid, &reg).unwrap();
        assert_eq!(eff_pow, 2,
            "Token should have effective power 2 (+1 from Intangible Virtue), got {}", eff_pow);
        assert_eq!(eff_tou, 2,
            "Token should have effective toughness 2 (+1 from Intangible Virtue), got {}", eff_tou);
        assert!(final_state.has_keyword(tid, Keyword::Vigilance, &reg),
            "Token should have vigilance from Intangible Virtue");
    }
}

// =====================================================================
// Scenario: Unruly Mob -- cast the creature
//
// P0 has Unruly Mob + 2 Plains. Cast the 1/1.
// Then kill a friendly creature to verify the +1/+1 counter trigger.
// =====================================================================

#[test]
fn scripted_tier3_unruly_mob() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 2;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let um_id = reg.get_id_by_name("Unruly Mob").unwrap();
    let um = state.create_object(um_id, P0, Zone::Hand, Some(1), Some(1));
    state.get_object_mut(um).unwrap().name = "Unruly Mob".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..2 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 2 Plains, cast Unruly Mob
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::CastSpell { object_id: um, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Unruly Mob, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Unruly Mob");

    // Verify outcome: Unruly Mob on the battlefield
    let mob_id = {
        let v: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter().filter(|o| o.name == "Unruly Mob").collect();
        assert_eq!(v.len(), 1, "Expected Unruly Mob on battlefield");
        v[0].id
    };

    // Kill a friendly creature to trigger Unruly Mob
    final_state.events.clear();
    let fodder = final_state.create_token("Fodder", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);
    final_state.get_object_mut(fodder).unwrap().damage_marked = 1;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    assert_eq!(final_state.get_counter_count(mob_id, CounterType::PlusOnePlusOne), 1,
        "Unruly Mob should get +1/+1 counter when ally dies");
}

// =====================================================================
// Scenario: Lumberknot -- cast the hexproof creature
//
// P0 has Lumberknot + 4 Forests. Cast the hexproof 1/1.
// Then kill a creature to verify the +1/+1 counter trigger.
// =====================================================================

#[test]
fn scripted_tier3_lumberknot() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 4;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let lk_id = reg.get_id_by_name("Lumberknot").unwrap();
    let lk = state.create_object(lk_id, P0, Zone::Hand, Some(1), Some(1));
    state.get_object_mut(lk).unwrap().name = "Lumberknot".into();

    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut forests = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        forests.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 4 Forests, cast Lumberknot
    let actions = vec![
        Action::ActivateManaAbility { object_id: forests[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: forests[3], ability_index: 0 },
        Action::CastSpell { object_id: lk, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Lumberknot, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Lumberknot");

    // Verify outcome: Lumberknot on the battlefield
    let lk_obj_id = {
        let v: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter().filter(|o| o.name == "Lumberknot").collect();
        assert_eq!(v.len(), 1, "Expected Lumberknot on battlefield");
        v[0].id
    };

    // Kill any creature to trigger Lumberknot
    final_state.events.clear();
    let fodder = final_state.create_token("Fodder", P1, 1, 1, vec![], vec![CardType::Creature], vec![]);
    final_state.get_object_mut(fodder).unwrap().damage_marked = 1;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    assert_eq!(final_state.get_counter_count(lk_obj_id, CounterType::PlusOnePlusOne), 1,
        "Lumberknot should get +1/+1 counter when any creature dies");
}

// =====================================================================
// Scenario: Elder Cathar -- cast the creature, then test the dies
// trigger with multiple targets (two buddy creatures).
//
// P0 has Elder Cathar + 3 Plains and TWO buddy creatures on
// the battlefield. When Elder Cathar dies, script choosing
// buddy1 to receive the +1/+1 counter.
// =====================================================================

#[test]
fn scripted_tier3_elder_cathar() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 3;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    // P0: Two buddy creatures so the dies trigger has 2+ targets
    let buddy1 = state.create_token("Buddy", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);
    state.get_object_mut(buddy1).unwrap().summoning_sick = false;
    let buddy2 = state.create_token("Buddy", P0, 1, 1, vec![], vec![CardType::Creature], vec![]);
    state.get_object_mut(buddy2).unwrap().summoning_sick = false;

    let ec_id = reg.get_id_by_name("Elder Cathar").unwrap();
    let ec = state.create_object(ec_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(ec).unwrap().name = "Elder Cathar".into();

    let plains_id = reg.get_id_by_name("Plains").unwrap();
    let mut plains = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(plains_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        plains.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Plains, cast Elder Cathar
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::CastSpell { object_id: ec, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Elder Cathar, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Elder Cathar");

    // Verify outcome: Elder Cathar on the battlefield
    let ec_obj_id = {
        let cathars: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter()
            .filter(|o| o.name == "Elder Cathar")
            .collect();
        assert_eq!(cathars.len(), 1,
            "Expected Elder Cathar on battlefield, found {}", cathars.len());
        cathars[0].id
    };

    // Kill Elder Cathar to trigger the dies ability with 2+ targets
    final_state.events.clear();
    final_state.get_object_mut(ec_obj_id).unwrap().damage_marked = 2;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);

    // The trigger should set a ResolutionChoice for P0 with 2+ targets.
    assert!(matches!(&final_state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice { player, .. }) if *player == P0),
        "Should have a ResolutionChoice for P0");

    // Script: choose buddy1 to get the counter.
    let choice_action = Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTarget(Some(Target::Object(buddy1))),
    };
    final_state = engine::submit_action(&final_state, &choice_action, &reg);
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);

    // Verify buddy1 got a +1/+1 counter
    let b1_counters = final_state.get_counter_count(buddy1, CounterType::PlusOnePlusOne);
    let b2_counters = final_state.get_counter_count(buddy2, CounterType::PlusOnePlusOne);
    assert!(b1_counters >= 1,
        "Elder Cathar should grant +1/+1 counter to buddy1 (buddy1={}, buddy2={})",
        b1_counters, b2_counters);
    assert_eq!(b2_counters, 0,
        "buddy2 should not have received a counter (buddy1={}, buddy2={})",
        b1_counters, b2_counters);
}

// =====================================================================
// Scenario: Village Cannibals -- cast the creature
//
// P0 has Village Cannibals + 3 Swamps. Cast the 2/2.
// Then kill a Human creature to verify the +1/+1 counter trigger.
// =====================================================================

#[test]
fn scripted_tier3_village_cannibals() {
    let reg = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 20;
    state.turn_number = 3;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.players[0].land_plays_remaining = 0;

    let vc_id = reg.get_id_by_name("Village Cannibals").unwrap();
    let vc = state.create_object(vc_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(vc).unwrap().name = "Village Cannibals".into();

    let swamp_id = reg.get_id_by_name("Swamp").unwrap();
    let mut swamps = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
        swamps.push(id);
    }

    add_libraries(&mut state, &reg);

    // Script: tap 3 Swamps, cast Village Cannibals
    let actions = vec![
        Action::ActivateManaAbility { object_id: swamps[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: swamps[2], ability_index: 0 },
        Action::CastSpell { object_id: vc, targets: vec![] },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }),
        "Should cast Village Cannibals, not {:?}", action);
    assert_eq!(spell_name(&final_state, &action), "Village Cannibals");

    // Verify outcome: Village Cannibals on the battlefield
    let vc_obj_id = {
        let cannibals: Vec<_> = final_state.objects_in_zone(Zone::Battlefield, P0)
            .into_iter()
            .filter(|o| o.name == "Village Cannibals")
            .collect();
        assert_eq!(cannibals.len(), 1,
            "Expected Village Cannibals on battlefield, found {}", cannibals.len());
        cannibals[0].id
    };

    // Kill a Human creature to trigger Village Cannibals
    final_state.events.clear();
    let dt_card_id = reg.get_id_by_name("Doomed Traveler").unwrap();
    let human = final_state.create_object(dt_card_id, P0, Zone::Battlefield, Some(1), Some(1));
    final_state.get_object_mut(human).unwrap().name = "Doomed Traveler".into();
    final_state.get_object_mut(human).unwrap().damage_marked = 1;
    check_state_based_actions_with_registry(&mut final_state, Some(&reg));
    triggers::process_triggers(&mut final_state, &reg);
    assert_eq!(final_state.get_counter_count(vc_obj_id, CounterType::PlusOnePlusOne), 1,
        "Village Cannibals should get +1/+1 counter when Human dies");
}

//! Deterministic scenario tests using ScriptedPlayer.
//!
//! These tests verify the ScriptedPlayer infrastructure works end-to-end
//! across the three distinct code paths:
//! 1. Mana tap → CastSpell (counterspell scenario)
//! 2. Combat decisions — DeclareBlockers (deathtouch block scenario)
//! 3. Mid-resolution choices — ResolveChoice (Slayer of the Wicked ETB)
//!
//! Individual card mechanics are already tested by the ~300 engine tests.
//! These exist to prove the Player trait interface works correctly.

use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::state::{AwaitingAction, CombatState, GameState};
use mtg_engine::triggers;
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

// ═══════════════════════════════════════════════════════════════════
// Path 1: Mana tap → CastSpell
//
// P1 has Counterspell in hand, 3 Islands untapped, Kalonian Tusker
// on the stack. Scripts tapping two Islands and casting Counterspell.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_counterspell() {
    let registry = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 14;
    state.turn_number = 5;
    state.active_player = P0;
    state.priority_player = Some(P1);
    state.step = Step::PrecombatMain;
    state.is_first_turn = false;
    state.consecutive_passes = 1;

    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let island_id = registry.get_id_by_name("Island").unwrap();
    let kalonian_tusker_id = registry.get_id_by_name("Kalonian Tusker").unwrap();
    let counterspell_id = registry.get_id_by_name("Counterspell").unwrap();

    // P0: 3 Mountains (tapped)
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
    state.stack.push(mtg_engine::state::StackEntry::Spell(tusker));

    // P1: 3 Islands (untapped)
    let mut islands = Vec::new();
    for _ in 0..3 {
        let id = state.create_object(island_id, P1, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Island".into();
        obj.summoning_sick = false;
        islands.push(id);
    }

    // P1 hand: Counterspell
    let counter = state.create_object(counterspell_id, P1, Zone::Hand, None, None);
    state.get_object_mut(counter).unwrap().name = "Counterspell".into();

    add_libraries(&mut state, &registry);

    // Script: tap Island, tap Island, cast Counterspell targeting Tusker.
    let actions = vec![
        Action::ActivateManaAbility { object_id: islands[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: islands[1], ability_index: 0 },
        Action::CastSpell {
            object_id: counter,
            targets: vec![Target::Object(tusker)],
        },
    ];
    let mut player = ScriptedPlayer::new("P1", actions);

    let (action, mut final_state) = run_scripted_decision(&state, P1, &mut player, &registry);

    assert!(matches!(&action, Action::CastSpell { .. }));
    assert_eq!(spell_name(&final_state, &action), "Counterspell");

    // Tusker should be countered (in graveyard).
    assert!(
        final_state.objects_in_zone(Zone::Graveyard, P0)
            .iter().any(|o| o.name == "Kalonian Tusker"),
        "Kalonian Tusker should be countered and in graveyard"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Path 2: Combat decision — DeclareBlockers
//
// P0 attacks with Kalonian Tusker (3/3). P1 has Typhoid Rats (1/1
// deathtouch). Scripts blocking to trade.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_deathtouch_block() {
    let registry = CardRegistry::with_all_cards();
    let mut state = GameState::new(2);
    state.players[0].life = 20;
    state.players[1].life = 8;
    state.turn_number = 6;
    state.active_player = P0;
    state.step = Step::DeclareBlockers;
    state.is_first_turn = false;

    let tusker_id = registry.get_id_by_name("Kalonian Tusker").unwrap();
    let rats_id = registry.get_id_by_name("Typhoid Rats").unwrap();
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let swamp_id = registry.get_id_by_name("Swamp").unwrap();

    // P0 attacker: Kalonian Tusker 3/3
    let tusker = state.create_object(tusker_id, P0, Zone::Battlefield, Some(3), Some(3));
    {
        let obj = state.get_object_mut(tusker).unwrap();
        obj.name = "Kalonian Tusker".into();
        obj.summoning_sick = false;
        obj.tapped = true;
        obj.colors = vec![Color::Green];
    }

    // Set up combat state
    let mut combat_state = CombatState::new();
    combat_state.attackers.insert(tusker, P1);
    combat_state.blocker_assignments.insert(tusker, Vec::new());
    state.combat = Some(combat_state);
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers {
        defending_player: P1,
    });
    state.priority_player = Some(P1);

    // P1: Typhoid Rats 1/1 deathtouch
    let rats = state.create_object(rats_id, P1, Zone::Battlefield, Some(1), Some(1));
    {
        let obj = state.get_object_mut(rats).unwrap();
        obj.name = "Typhoid Rats".into();
        obj.summoning_sick = false;
        obj.colors = vec![Color::Black];
    }

    // Lands and libraries
    for _ in 0..3 {
        let id = state.create_object(swamp_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }
    add_libraries(&mut state, &registry);

    // Script: declare Typhoid Rats blocking Kalonian Tusker
    let mut player = ScriptedPlayer::new("Blocker", vec![
        Action::DeclareBlockers { assignments: vec![(rats, tusker)] },
    ]);

    let legal = engine::legal_actions(&state, &registry);
    assert!(legal.combat_prompt.is_some());

    let action = player.choose_combat(legal.combat_prompt.as_ref().unwrap());

    if let Action::DeclareBlockers { .. } = &action {
        let mut new_state = engine::submit_action(&state, &action, &registry);
        combat::deal_combat_damage(&mut new_state, &registry);
        while check_state_based_actions_with_registry(&mut new_state, Some(&registry)) {}

        // Both should be dead — deathtouch trade.
        assert_eq!(new_state.get_object(tusker).unwrap().zone, Zone::Graveyard,
            "Tusker should die from deathtouch");
        assert_eq!(new_state.get_object(rats).unwrap().zone, Zone::Graveyard,
            "Rats should die from combat damage");
    } else {
        panic!("Expected DeclareBlockers, got: {:?}", action);
    }
}

// ═══════════════════════════════════════════════════════════════════
// Path 3: Mid-resolution choice — ResolveChoice
//
// P0 casts Slayer of the Wicked. Opponent has TWO Walking Corpses
// (Zombies), so the ETB trigger presents a choice. Scripts choosing
// the first one.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scripted_resolution_choice() {
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

    // Opponent has TWO Walking Corpses so there are 2+ ETB targets
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

    // Script: tap 4 Plains, cast Slayer, then choose wc1 as ETB target
    let actions = vec![
        Action::ActivateManaAbility { object_id: plains[0], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[1], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[2], ability_index: 0 },
        Action::ActivateManaAbility { object_id: plains[3], ability_index: 0 },
        Action::CastSpell { object_id: sw, targets: vec![] },
        Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(wc1))),
        },
    ];
    let mut player = ScriptedPlayer::new("P0", actions);

    let (action, final_state) = run_scripted_decision(&state, P0, &mut player, &reg);

    assert!(matches!(&action, Action::CastSpell { .. }));
    assert_eq!(spell_name(&final_state, &action), "Slayer of the Wicked");

    // Slayer on battlefield
    assert_eq!(
        final_state.objects_in_zone(Zone::Battlefield, P0)
            .iter().filter(|o| o.name == "Slayer of the Wicked").count(),
        1
    );

    // First Walking Corpse destroyed, second still alive
    assert_eq!(final_state.get_object(wc1).unwrap().zone, Zone::Graveyard,
        "Chosen Walking Corpse should be destroyed");
    assert_eq!(final_state.get_object(wc2).unwrap().zone, Zone::Battlefield,
        "Unchosen Walking Corpse should still be alive");
}

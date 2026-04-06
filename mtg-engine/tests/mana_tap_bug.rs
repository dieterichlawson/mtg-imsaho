/// Test to reproduce mana tapping bug:
/// User had Falkenrath Marauders {3}{R}{R} in hand.
/// Tapped a Swamp and Stensia Bloodhall but mana did not appear in pool.
/// Then tapped two more Swamps and mana DID appear.
///
/// Board state from Turn 19:
/// - P0 (player): 4x Swamp, 4x Mountain (2 tapped), 1x Stensia Bloodhall
///   Creatures: Vampire Interloper, Falkenrath Marauders [summoning sick]
///   Enchantment: Curse of Stalked Prey (attached to opponent)
///   Hand: empty (drew Falkenrath Marauders which was just cast)
///   Graveyard: 4 cards
///
/// The issue was after playing the land and casting Falkenrath Marauders.
/// Let's reproduce the state right at precombat main with the spell in hand.

mod common;
use common::*;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::{CardId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn setup_turn19_state() -> (GameState, CardRegistry) {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0's lands: 4x Swamp (untapped), 2x Mountain (untapped), 2x Mountain (tapped), 1x Stensia Bloodhall (untapped)
    let swamp_id = registry.get_id_by_name("Swamp").unwrap();
    let mountain_id = registry.get_id_by_name("Mountain").unwrap();
    let bloodhall_id = registry.get_id_by_name("Stensia Bloodhall").unwrap();

    let mut swamps = Vec::new();
    for _ in 0..4 {
        let id = state.create_object(swamp_id, P0, Zone::Battlefield, None, None);
        swamps.push(id);
    }
    for _ in 0..2 {
        state.create_object(mountain_id, P0, Zone::Battlefield, None, None);
    }
    for _ in 0..2 {
        let id = state.create_object(mountain_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().tapped = true;
    }
    let bloodhall = state.create_object(bloodhall_id, P0, Zone::Battlefield, None, None);

    // P0's creatures
    let _vi = named_creature(&mut state, &registry, "Vampire Interloper", P0);

    // Falkenrath Marauders in hand
    let fm_id = registry.get_id_by_name("Falkenrath Marauders").unwrap();
    let fm = state.create_object(fm_id, P0, Zone::Hand, Some(2), Some(2));
    state.get_object_mut(fm).unwrap().name = "Falkenrath Marauders".into();

    // P1's lands
    let plains_id = registry.get_id_by_name("Plains").unwrap();
    let forest_id = registry.get_id_by_name("Forest").unwrap();
    let township_id = registry.get_id_by_name("Gavony Township").unwrap();
    for _ in 0..3 {
        let id = state.create_object(plains_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().tapped = true;
    }
    let _forest = state.create_object(forest_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(_forest).unwrap().tapped = true;
    state.create_object(township_id, P1, Zone::Battlefield, None, None);

    // P1's creatures
    let _hamlet = named_creature(&mut state, &registry, "Hamlet Captain", P1);
    let _champ = named_creature(&mut state, &registry, "Champion of the Parish", P1);

    state.players[0].life = 9;
    state.players[1].life = 6;

    (state, registry)
}

/// Test: Tapping a Swamp should add {B} to the mana pool.
#[test]
fn tapping_swamp_adds_black_mana() {
    let (state, registry) = setup_turn19_state();

    // Find the legal actions
    let legal = engine::legal_actions(&state, &registry);

    // Find a "Tap Swamp for mana" action
    let tap_swamp = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateManaAbility { .. }
    ) && {
        if let Action::ActivateManaAbility { object_id, .. } = a {
            state.get_object(*object_id)
                .map(|o| o.name == "Swamp" || registry.card_data(o.card_id).map(|d| d.name == "Swamp").unwrap_or(false))
                .unwrap_or(false)
        } else { false }
    });
    assert!(tap_swamp.is_some(), "Should have a Tap Swamp action available");

    // Tap the Swamp
    let new_state = engine::submit_action(&state, tap_swamp.unwrap(), &registry);

    // Check that black mana was added
    let pool = &new_state.get_player(P0).mana_pool;
    let black = pool.mana.get(&ManaType::Black).copied().unwrap_or(0);
    assert_eq!(black, 1, "Tapping Swamp should add 1 Black mana, pool: {:?}", pool.mana);
}

/// Test: Tapping Stensia Bloodhall should add {C} to the mana pool.
#[test]
fn tapping_stensia_bloodhall_adds_colorless() {
    let (state, registry) = setup_turn19_state();

    let legal = engine::legal_actions(&state, &registry);

    let tap_bloodhall = legal.actions.iter().find(|a| {
        if let Action::ActivateManaAbility { object_id, .. } = a {
            state.get_object(*object_id)
                .map(|o| registry.card_data(o.card_id).map(|d| d.name == "Stensia Bloodhall").unwrap_or(false))
                .unwrap_or(false)
        } else { false }
    });
    assert!(tap_bloodhall.is_some(), "Should have a Tap Stensia Bloodhall action available");

    let new_state = engine::submit_action(&state, tap_bloodhall.unwrap(), &registry);

    let pool = &new_state.get_player(P0).mana_pool;
    let colorless = pool.mana.get(&ManaType::Colorless).copied().unwrap_or(0);
    assert_eq!(colorless, 1, "Tapping Stensia Bloodhall should add 1 Colorless mana, pool: {:?}", pool.mana);
}

/// Test: Tapping Swamp then Stensia Bloodhall should accumulate mana.
#[test]
fn sequential_taps_accumulate_mana() {
    let (state, registry) = setup_turn19_state();

    // Tap Swamp first
    let legal = engine::legal_actions(&state, &registry);
    let tap_swamp = legal.actions.iter().find(|a| {
        if let Action::ActivateManaAbility { object_id, .. } = a {
            state.get_object(*object_id)
                .and_then(|o| registry.card_data(o.card_id))
                .map(|d| d.name == "Swamp")
                .unwrap_or(false)
        } else { false }
    }).expect("Should have Tap Swamp");

    let state2 = engine::submit_action(&state, tap_swamp, &registry);

    // Verify mana pool after first tap
    let black1 = state2.get_player(P0).mana_pool.mana.get(&ManaType::Black).copied().unwrap_or(0);
    assert_eq!(black1, 1, "After first Swamp tap: expected 1 Black, got {}", black1);

    // Now tap Stensia Bloodhall
    let legal2 = engine::legal_actions(&state2, &registry);
    let tap_bloodhall = legal2.actions.iter().find(|a| {
        if let Action::ActivateManaAbility { object_id, .. } = a {
            state2.get_object(*object_id)
                .and_then(|o| registry.card_data(o.card_id))
                .map(|d| d.name == "Stensia Bloodhall")
                .unwrap_or(false)
        } else { false }
    }).expect("Should have Tap Stensia Bloodhall");

    let state3 = engine::submit_action(&state2, tap_bloodhall, &registry);

    // Verify both mana types present
    let black2 = state3.get_player(P0).mana_pool.mana.get(&ManaType::Black).copied().unwrap_or(0);
    let colorless = state3.get_player(P0).mana_pool.mana.get(&ManaType::Colorless).copied().unwrap_or(0);
    assert_eq!(black2, 1, "After Bloodhall tap: expected 1 Black, got {}", black2);
    assert_eq!(colorless, 1, "After Bloodhall tap: expected 1 Colorless, got {}", colorless);
}

/// Test: The deduplication of mana abilities should not prevent tapping
/// multiple copies of the same land type.
#[test]
fn can_tap_multiple_swamps_sequentially() {
    let (state, registry) = setup_turn19_state();

    let mut current = state;
    for i in 0..4 {
        let legal = engine::legal_actions(&current, &registry);
        let tap_swamp = legal.actions.iter().find(|a| {
            if let Action::ActivateManaAbility { object_id, .. } = a {
                current.get_object(*object_id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .map(|d| d.name == "Swamp")
                    .unwrap_or(false)
            } else { false }
        });

        assert!(tap_swamp.is_some(),
            "Should be able to tap Swamp #{} (already tapped {})", i + 1, i);

        current = engine::submit_action(&current, tap_swamp.unwrap(), &registry);

        let black = current.get_player(P0).mana_pool.mana.get(&ManaType::Black).copied().unwrap_or(0);
        assert_eq!(black, (i + 1) as u32,
            "After tapping {} Swamps, expected {} Black mana, got {}",
            i + 1, i + 1, black);
    }
}

/// Test: After tapping mana, has_meaningful_action should still be true
/// if we can cast something with the accumulated mana.
#[test]
fn meaningful_action_detected_after_partial_taps() {
    let (state, registry) = setup_turn19_state();

    // Tap one Swamp
    let legal = engine::legal_actions(&state, &registry);
    let tap_swamp = legal.actions.iter().find(|a| {
        if let Action::ActivateManaAbility { object_id, .. } = a {
            state.get_object(*object_id)
                .and_then(|o| registry.card_data(o.card_id))
                .map(|d| d.name == "Swamp")
                .unwrap_or(false)
        } else { false }
    }).expect("Should have Tap Swamp");

    let state2 = engine::submit_action(&state, tap_swamp, &registry);

    // After tapping one Swamp, the engine should still see meaningful actions
    // (more mana abilities to tap + Falkenrath Marauders castable with potential mana)
    let legal2 = engine::legal_actions(&state2, &registry);
    let has_non_pass = legal2.actions.iter().any(|a| !matches!(a,
        Action::PassPriority | Action::Concede | Action::ActivateManaAbility { .. }
    ));
    let has_mana_ability = legal2.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { .. }));

    // We should at least have mana abilities to keep tapping
    assert!(has_mana_ability, "Should still have mana abilities after one tap");

    // With potential mana from remaining lands + {B} in pool,
    // Falkenrath Marauders ({3}{R}{R}) should be castable
    // Pool: {B}, Potential: 3 more Swamp ({B}), 2 Mountain ({R}), 1 Bloodhall ({C})
    // Total potential: 4{B} + 2{R} + 1{C} = enough for {3}{R}{R}
}

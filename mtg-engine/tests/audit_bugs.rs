//! Failing tests that demonstrate bugs found by the Sonnet 4.6 audit.
//! Each test documents a specific issue and is expected to FAIL until the bug is fixed.

mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

// ═══════════════════════════════════════════════════════════════
// ENGINE: SUMMONING SICKNESS
// Tap abilities should not be activatable on the turn a creature enters.
// ═══════════════════════════════════════════════════════════════

/// Bug: Avacynian Priest can activate {1}, {T} ability on the turn it enters.
/// The engine checks `requires_tap && obj_tapped` (line 356) but never checks
/// `summoning_sick`. Per MTG rules, creatures with summoning sickness cannot
/// use abilities with {T} in the cost.
#[test]
fn bug_summoning_sickness_not_enforced_for_tap_abilities() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Avacynian Priest with summoning sickness (just entered this turn)
    let priest = {
        let card_id = registry.get_id_by_name("Avacynian Priest").unwrap();
        let data = registry.card_data(card_id).unwrap();
        let id = state.create_object(card_id, P0, Zone::Battlefield, data.power, data.toughness);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Avacynian Priest".into();
        // summoning_sick defaults to true on creation — do NOT clear it
        id
    };

    // Verify it has summoning sickness
    assert!(state.get_object(priest).unwrap().summoning_sick,
        "Priest should have summoning sickness");

    // Add mana for the {1} activation cost
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Place a target creature for the opponent
    let _target = ready_creature(&mut state, P1, 3, 3);

    // Get legal actions — the Priest's tap ability should NOT be available
    let legal = engine::legal_actions(&state, &registry);
    let has_priest_ability = legal.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == priest)
    });

    // BUG: This assertion should pass (ability should NOT be available)
    // but currently fails because engine doesn't check summoning sickness for tap abilities
    assert!(!has_priest_ability,
        "Priest with summoning sickness should NOT be able to activate {{T}} ability");
}

// ═══════════════════════════════════════════════════════════════
// SUBTYPE CHECK MISSES TOKENS
// Cards that check subtypes via registry.card_data() miss tokens,
// which store subtypes on obj.subtypes instead.
// ═══════════════════════════════════════════════════════════════

/// Bug: Victim of Night can target Vampire tokens.
/// Oracle: "Destroy target non-Vampire, non-Werewolf, non-Zombie creature."
/// The is_valid_target check uses registry.card_data() which returns None for
/// tokens, so the subtype exclusion fails and tokens are targetable.
#[test]
fn bug_victim_of_night_can_target_vampire_token() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a Vampire token (like Bloodline Keeper creates)
    let vampire_token = state.create_token_with_subtypes(
        "Vampire", P1, 2, 2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        vec!["Vampire".into()],
    );
    if let Some(obj) = state.get_object_mut(vampire_token) {
        obj.summoning_sick = false;
    }

    // Verify token has Vampire subtype
    assert!(state.get_object(vampire_token).unwrap().subtypes.contains(&"Vampire".into()),
        "Token should have Vampire subtype");

    // Cast Victim of Night targeting the Vampire token
    let victim = castable_spell(&mut state, &registry, "Victim of Night", P0);

    // Check if the Vampire token is a valid target
    let behavior = registry.get(
        registry.get_id_by_name("Victim of Night").unwrap()
    ).unwrap();
    let is_valid = behavior.is_valid_target(
        &state, P0, &Target::Object(vampire_token), &registry
    );

    // BUG: Token should NOT be a valid target (it's a Vampire),
    // but is_valid_target only checks registry which has no data for tokens
    assert!(!is_valid,
        "Vampire token should NOT be a valid target for Victim of Night");
}

// ═══════════════════════════════════════════════════════════════
// ENGINE: TRIGGER ZONE CHECK
// ETB triggers should still resolve if the source leaves before resolution.
// Per MTG rules, an ETB trigger goes on the stack and resolves independently.
// ═══════════════════════════════════════════════════════════════

/// Bug: ETB triggers are suppressed when source leaves battlefield before resolution.
/// The trigger resolution in triggers.rs:893-899 checks zone == Battlefield.
/// Per MTG rules, ETB triggers resolve independently — removing the source
/// doesn't prevent the trigger from resolving.
///
/// This test goes through the trigger dispatch system (not calling handler directly)
/// to demonstrate the bug is in the trigger resolution path.
#[test]
fn bug_etb_trigger_suppressed_when_source_leaves() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 some library cards to mill
    for _ in 0..10 {
        let card = state.create_object(
            registry.get_id_by_name("Grizzly Bears").unwrap(),
            P0, Zone::Library, Some(2), Some(2),
        );
        state.get_player_mut(P0).library_order.push(card);
    }
    let lib_before = state.get_player(P0).library_order.len();

    // Cast Armored Skaab — this will put it on the stack
    let skaab = castable_spell(&mut state, &registry, "Armored Skaab", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: skaab, targets: vec![], sacrifice: None, exile_count: None, alternative_cost: None },
        &registry,
    );
    // Resolve — moves to battlefield, queues ETB trigger
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Skaab is now on battlefield with ETB trigger pending
    assert_eq!(state.get_object(skaab).unwrap().zone, Zone::Battlefield);

    // Kill Skaab before the ETB trigger resolves (move to graveyard)
    state.move_object(skaab, Zone::Graveyard);
    assert_eq!(state.get_object(skaab).unwrap().zone, Zone::Graveyard);

    // Process pending triggers — the ETB mill should still happen
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    let lib_after = state.get_player(P0).library_order.len();

    // BUG: Mill doesn't happen because trigger resolution checks zone == Battlefield
    assert_eq!(lib_before - lib_after, 4,
        "ETB trigger should still mill 4 even after Skaab left the battlefield");
}

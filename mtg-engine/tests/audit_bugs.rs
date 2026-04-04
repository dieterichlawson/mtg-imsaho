//! Failing tests that demonstrate bugs found by the Sonnet 4.6 audit.
//! Each test documents a specific issue and is expected to FAIL until the bug is fixed.

mod common;
use common::*;

use mtg_engine::actions::Action;
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

//! Tests for summoning sickness rules (rule 302.6).

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

/// Rule 302.6: Summoning sickness clears at the beginning of your untap step.
#[test]
fn summoning_sickness_clears_at_own_untap() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    let creature = sick_creature(&mut state, P0, 3, 3);
    assert!(state.get_object(creature).unwrap().summoning_sick);

    // Advance to P1's untap — P0's creature should still be sick.
    engine::advance_step(&mut state, &registry);
    assert_eq!(state.active_player, P1);
    assert!(state.get_object(creature).unwrap().summoning_sick,
        "Creature should still be sick during opponent's untap");

    // Advance through P1's entire turn to get back to P0's untap.
    loop {
        engine::advance_step(&mut state, &registry);
        if state.step == Step::Untap && state.active_player == P0 {
            break;
        }
    }

    assert!(!state.get_object(creature).unwrap().summoning_sick,
        "Creature should no longer be sick after controller's untap step");
}

/// Summoning sickness is set when a creature enters the battlefield
/// from any zone.
#[test]
fn entering_battlefield_gives_summoning_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Battlefield, &registry);
    assert!(state.get_object(creature).unwrap().summoning_sick);
}

/// Leaving and re-entering the battlefield resets summoning sickness.
#[test]
fn re_entering_battlefield_resets_summoning_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Hand, &registry);
    state.move_object(creature, Zone::Battlefield, &registry);
    assert!(state.get_object(creature).unwrap().summoning_sick,
        "Should be sick again after re-entering battlefield");
}

/// Summoning sickness prevents attacking but NOT blocking.
#[test]
fn sick_creature_cant_attack_but_can_block() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);

    assert!(!combat::eligible_attackers(&state, P0, &reg).contains(&creature),
        "Sick creature should not be able to attack");
    assert!(combat::eligible_blockers(&state, P0, &reg).contains(&creature),
        "Sick creature should be able to block");
}

/// Summoning sickness is cleared when leaving the battlefield.
#[test]
fn leaving_battlefield_clears_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);
    assert!(state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Graveyard, &registry);
    assert!(!state.get_object(creature).unwrap().summoning_sick,
        "Summoning sickness should be cleared when leaving battlefield");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

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

    assert!(!has_priest_ability,
        "Priest with summoning sickness should NOT be able to activate {{T}} ability");
}

/// CR 613.10c-adjacent bookkeeping: "gaining" control of a permanent you
/// already control is not a control change — it must not re-apply the
/// summoning-sickness reset that a real change of controller causes.
#[test]
fn a_control_change_to_the_same_controller_is_a_no_op() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let veteran = ready_creature(&mut state, P0, 2, 2);
    assert!(!state.get_object(veteran).unwrap().summoning_sick);

    state.change_control(veteran, P0);

    assert!(!state.get_object(veteran).unwrap().summoning_sick,
        "no controller changed, so no summoning sickness");
    let _ = reg;
}

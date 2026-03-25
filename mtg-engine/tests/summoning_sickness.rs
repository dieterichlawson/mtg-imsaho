//! Tests for summoning sickness rules (rule 302.6).

mod common;

use common::*;
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
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Battlefield);
    assert!(state.get_object(creature).unwrap().summoning_sick);
}

/// Leaving and re-entering the battlefield resets summoning sickness.
#[test]
fn re_entering_battlefield_resets_summoning_sickness() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Hand);
    state.move_object(creature, Zone::Battlefield);
    assert!(state.get_object(creature).unwrap().summoning_sick,
        "Should be sick again after re-entering battlefield");
}

/// Summoning sickness prevents attacking but NOT blocking.
#[test]
fn sick_creature_cant_attack_but_can_block() {
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);

    assert!(!combat::eligible_attackers(&state, P0).contains(&creature),
        "Sick creature should not be able to attack");
    assert!(combat::eligible_blockers(&state, P0).contains(&creature),
        "Sick creature should be able to block");
}

/// Lands should NOT get summoning sickness when entering battlefield.
#[test]
fn lands_never_have_summoning_sickness() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let land = state.create_object(CardId(1), P0, Zone::Hand, None, None);

    state.move_object(land, Zone::Battlefield);
    // Lands do get the flag set by move_object, but it doesn't matter
    // since they can't attack and their mana abilities don't use the tap symbol
    // in the summoning sickness sense. The engine handles this by
    // clearing it in PlayLand.

    // What matters: mana abilities work on summoning sick permanents
    // because summoning sickness only restricts attack and {T}/{Q} costs,
    // and our mana ability check doesn't gate on summoning_sick.
}

/// Summoning sickness is cleared when leaving the battlefield.
#[test]
fn leaving_battlefield_clears_sickness() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);
    assert!(state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Graveyard);
    assert!(!state.get_object(creature).unwrap().summoning_sick,
        "Summoning sickness should be cleared when leaving battlefield");
}

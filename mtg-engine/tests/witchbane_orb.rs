//! Tests for Witchbane Orb.
//!
//! Oracle: {4} Artifact
//! When Witchbane Orb enters the battlefield, destroy all Curses attached to you.
//! You have hexproof.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::types::*;
/// Player has hexproof when they control Witchbane Orb.
#[test]
fn grants_player_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    assert!(!state.player_has_hexproof(P0, &reg), "Should not have hexproof without Orb");

    let _orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    assert!(state.player_has_hexproof(P0, &reg), "Should have hexproof with Orb");
    assert!(!state.player_has_hexproof(P1, &reg), "Opponent should not have hexproof");
}

/// Opponent cannot target a player with hexproof.
#[test]
fn opponent_cannot_target_hexproof_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);

    // P0 has Witchbane Orb.
    let _orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    // P1 tries to cast a player-targeting spell (like Bump in the Night) at P0.
    // Bump in the Night: {B} Sorcery - Target player loses 3 life.
    let bump = castable_spell(&mut state, &reg, "Bump in the Night", P1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let cast_at_p0: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, targets, .. } = a {
            object_id == &bump && targets.iter().any(|t| {
                matches!(t, mtg_engine::actions::Target::Player(p) if *p == P0)
            })
        } else {
            false
        }
    }).collect();

    assert!(cast_at_p0.is_empty(),
        "Should not be able to target hexproof player");
}

/// Player can still target themselves even with hexproof (hexproof only prevents opponents).
#[test]
fn can_target_self_with_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has Witchbane Orb.
    let _orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    // P0 tries to cast Dream Twist (targets any player) at themselves.
    let twist = castable_spell(&mut state, &reg, "Dream Twist", P0);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let cast_at_p0: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, targets, .. } = a {
            object_id == &twist && targets.iter().any(|t| {
                matches!(t, mtg_engine::actions::Target::Player(p) if *p == P0)
            })
        } else {
            false
        }
    }).collect();

    assert!(!cast_at_p0.is_empty(),
        "Player should be able to target themselves even with hexproof");
}

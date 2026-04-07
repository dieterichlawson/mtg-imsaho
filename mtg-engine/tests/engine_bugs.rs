//! Tests for remaining engine bugs #5-10.
//!
//! #5: APNAP trigger ordering — architectural, not fixable without stack-based triggers
//! #6: Cleanup step SBA loop — SBAs should be checked after effects expire
//! #7: Mana emptying at step boundaries — already correct (tested in turn_structure.rs)
//! #8: Sorcery-speed empty stack check — already correct (tested in spells.rs)
//! #9: Zone change object identity — works correctly due to zone-based aura tracking
//! #10: Redundant resolve_top_of_stack call — wasteful but not incorrect

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ════════════════════════════════════════════════════════════════════
// Bug #6: Cleanup Step SBA Loop (CR 514.3a)
//
// After the cleanup step removes "until end of turn" effects and clears
// damage, SBAs should be checked. If any SBA fires (e.g., a creature
// now has 0 toughness), players get priority and there's another cleanup.
//
// Currently the cleanup step does NOT check SBAs.
// ════════════════════════════════════════════════════════════════════

/// A creature kept alive only by a +0/+1 until-end-of-turn effect that
/// has a -1/-1 counter should die during cleanup when the buff expires.
///
/// Setup: 1/1 creature with one -1/-1 counter and +0/+1 until EOT.
/// Effective toughness during the turn: 1 - 1 + 1 = 1 (alive).
/// During cleanup: buff expires → effective toughness: 1 - 1 = 0 → dies.
/// (Damage is also cleared, but that doesn't help a 0-toughness creature.)
#[test]
fn cleanup_sba_kills_creature_when_buff_expires() {
    let reg = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    let creature = ready_creature(&mut state, P0, 1, 1);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 1);

    // +0/+1 until end of turn keeps it alive.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 0,
            toughness_mod: 1,
        },
    );

    // Verify it's alive during the turn.
    assert_eq!(state.effective_toughness(creature, &reg), Some(1));
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should be alive during the turn (effective toughness 1)");

    // Fill libraries to avoid empty-library loss.
    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    // Advance to cleanup step.
    loop {
        engine::advance_step(&mut state, &reg);
        if state.step == Step::Cleanup {
            break;
        }
    }

    // After cleanup: EOT buff expired, SBAs ran and found 0 toughness.
    // The creature should have been killed during cleanup (CR 514.3a).
    // (Moving to graveyard clears counters, so we check zone, not counters.)
    assert!(state.until_end_of_turn.is_empty(), "EOT effects should be cleared");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature with 0 toughness after buff expires should die during cleanup (CR 514.3a)");
}

/// A creature that has enough toughness after EOT effects expire should survive.
/// (Verify the SBA check doesn't kill things that shouldn't die.)
#[test]
fn cleanup_sba_creature_survives_when_still_healthy() {
    let reg = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);

    // +2/+2 until end of turn. After cleanup: base 3/3, no damage. Survives.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 2,
            toughness_mod: 2,
        },
    );

    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    loop {
        engine::advance_step(&mut state, &reg);
        if state.step == Step::Cleanup {
            break;
        }
    }

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "3/3 creature with no damage should survive cleanup");
}

/// A creature with lethal damage but a toughness buff should survive during
/// the turn, then during cleanup both the buff AND the damage are cleared.
/// The creature should survive (damage cleared saves it even though buff expired).
#[test]
fn cleanup_clears_damage_and_buffs_simultaneously() {
    let reg = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    // 2/2 creature with 2 damage and +0/+1 until EOT.
    // During turn: effective toughness 3, damage 2 → alive.
    // Cleanup: buff expires AND damage clears → effective toughness 2, damage 0 → alive.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().damage_marked = 2;
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 0,
            toughness_mod: 1,
        },
    );

    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    loop {
        engine::advance_step(&mut state, &reg);
        if state.step == Step::Cleanup {
            break;
        }
    }

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive cleanup: damage AND buff both clear, leaving a 2/2 with 0 damage");
}

// ════════════════════════════════════════════════════════════════════
// Bug #10: Redundant resolve_top_of_stack Call
//
// The game loop calls resolve_top_of_stack on a clone (discarding the
// result), then clones the state again and resolves for real.
// This is wasteful — the first call does nothing useful.
// ════════════════════════════════════════════════════════════════════

/// Bug #10 is a redundant `resolve_top_of_stack` call on a discarded clone.
/// It wastes work but doesn't cause incorrect behavior because the clone
/// is discarded. This test just verifies that spells still resolve correctly
/// through the game loop (the redundant call doesn't break anything).
///
/// The fix is simply deleting the redundant line — no behavioral change.
#[test]
fn spell_resolves_correctly_through_game_loop() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 at 10 life, P0 will bolt them to 7.
    state.players[1].life = 10;

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    let mut cast = false;
    let bolt_id = bolt;
    let mut action_count = 0;

    engine::run_game_loop(&mut state, &reg, |_game_state, player, _legal| {
        action_count += 1;
        if action_count > 30 {
            return mtg_engine::actions::Action::Concede;
        }

        // P0 casts bolt targeting P1 on their first action.
        if player == P0 && !cast {
            cast = true;
            return mtg_engine::actions::Action::CastSpell {
                object_id: bolt_id,
                targets: vec![mtg_engine::actions::Target::Player(P1)],
                sacrifice: None,
                exile_count: None, exile_ids: vec![], alternative_cost: None,
            };
        }

        mtg_engine::actions::Action::PassPriority
    });

    // Bolt should have dealt exactly 3 damage (10 → 7), not 6 (10 → 4).
    assert_eq!(state.players[1].life, 7,
        "Lightning Bolt should deal exactly 3 damage through the game loop, not 6");
}

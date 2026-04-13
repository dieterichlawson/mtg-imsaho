//! Tests for Kessig Wolf Run.
//!
//! Oracle: Land
//! {T}: Add {C}.
//! {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Can activate with {R}{G} and X=0 (minimum).
#[test]
fn can_activate_with_rg_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Add just {R}{G} — X will be 0.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_activate = actions.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { ability_index: 1, .. })
    });

    assert!(can_activate,
        "Should be able to activate with just RG (X=0)");
}

/// Cannot activate without at least {R}{G}.
#[test]
fn cannot_activate_without_rg() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);
    let _creature = ready_creature(&mut state, P0, 2, 2);

    // Only {R} — no green.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_activate = actions.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { ability_index: 1, .. })
    });

    assert!(!can_activate,
        "Should not be able to activate without both R and G");
}

/// X=3 with 5 mana gives +3/+0 and trample.
#[test]
fn x_equals_3_gives_plus_3() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Add {R}{G} + 3 colorless = 5 mana total. X should be 3.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let action = Action::ActivateAbility {
        object_id: wolf_run,
        ability_index: 1,
        targets: vec![mtg_engine::actions::Target::Object(creature)],
        tap_plan: vec![],
        sacrifice: None,
        x_value: None,
    };

    state = mtg_engine::engine::submit_action(&state, &action, &reg);

    // Check +3/+0 effect.
    let power = state.effective_power(creature, &reg).unwrap_or(0);
    assert_eq!(power, 5,
        "Creature should have 2 + 3 = 5 power (got {})", power);

    // Check trample keyword.
    let has_trample = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantKeyword { target, keyword } if *target == creature && *keyword == Keyword::Trample));
    assert!(has_trample, "Creature should have trample");

    // All mana should be spent.
    assert!(state.get_player(P0).mana_pool.is_empty(),
        "All mana should be spent");
}

/// X=0 gives just trample (no power boost).
#[test]
fn x_equals_0_gives_trample_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Just {R}{G} — X = 0.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let action = Action::ActivateAbility {
        object_id: wolf_run,
        ability_index: 1,
        targets: vec![mtg_engine::actions::Target::Object(creature)],
        tap_plan: vec![],
        sacrifice: None,
        x_value: None,
    };

    state = mtg_engine::engine::submit_action(&state, &action, &reg);

    // Power should remain 2 (X=0 gives +0/+0).
    let power = state.effective_power(creature, &reg).unwrap_or(0);
    assert_eq!(power, 2, "Creature should still have 2 power with X=0");

    // Should still have trample.
    let has_trample = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantKeyword { target, keyword } if *target == creature && *keyword == Keyword::Trample));
    assert!(has_trample, "Creature should have trample even with X=0");
}

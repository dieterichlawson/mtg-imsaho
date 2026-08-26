mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Regression test: casting an X-cost spell (Devil's Play) with untapped lands
/// but no mana in pool should autotap the lands, not panic.
#[test]
fn test_x_cost_spell_autotap_does_not_panic() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Devil's Play ({X}{R}) in hand
    let devils_play = spell_in_hand(&mut state, &registry, "Devil's Play", P0);

    // Put 3 Mountains on the battlefield (enough for X=2 with {R} colored cost)
    named_permanent(&mut state, &registry, "Mountain", P0);
    named_permanent(&mut state, &registry, "Mountain", P0);
    named_permanent(&mut state, &registry, "Mountain", P0);

    // Put a target creature for the opponent
    let _target_creature = ready_creature(&mut state, P1, 2, 2);

    // Get legal actions — Devil's Play should be castable
    let legal = engine::legal_actions(&state, &registry);

    // Find the CastSpell action for Devil's Play
    let cast_action = legal
        .actions
        .iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == devils_play))
        .expect("Devil's Play should be castable with 3 untapped Mountains");

    // This should NOT panic — it previously panicked with "InsufficientMana"
    // because the tap_plan was empty for X-cost spells.
    let new_state = engine::submit_action(&state, cast_action, &registry);

    // Rules-strict X-cost casting: the spell stays in hand until the
    // ChooseXFunding prompt is resolved. After that, it's on the stack.
    let new_state = resolve_funding_max(&new_state, &registry);
    assert!(
        new_state
            .objects_in_zone(Zone::Stack, P0)
            .iter()
            .any(|o| o.name == "Devil's Play"),
        "Devil's Play should be on the stack after funding completes"
    );
}

/// Test that X-cost spell computes the correct X value when autotapping.
#[test]
fn test_x_cost_spell_correct_x_value() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils_play = spell_in_hand(&mut state, &registry, "Devil's Play", P0);

    // 5 Mountains: {R} for colored cost + 4 remaining = X=4
    for _ in 0..5 {
        named_permanent(&mut state, &registry, "Mountain", P0);
    }

    let target_creature = ready_creature(&mut state, P1, 5, 5);

    let legal = engine::legal_actions(&state, &registry);
    let cast_action = legal
        .actions
        .iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == devils_play))
        .expect("Devil's Play should be castable");

    let mut new_state = engine::submit_action(&state, cast_action, &registry);

    // After casting, the engine should present a ChooseXFunding prompt.
    // Submit a max-X response: tap all 4 remaining Mountains.
    new_state = resolve_funding_max(&new_state, &registry);

    // Check X value on the spell
    let stack_objs = new_state.objects_in_zone(Zone::Stack, P0);
    let spell_obj = stack_objs
        .iter()
        .find(|o| o.name == "Devil's Play")
        .expect("Devil's Play should be on the stack");
    let x_val = spell_obj.x_value;

    // X should be total mana (5) minus non-X cost (1 for {R}) = 4
    assert_eq!(
        x_val,
        Some(4),
        "X value should be 4 (5 mountains - 1 for {{R}})"
    );

    // Resolve and check damage
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &registry);
    let target = new_state.get_object(target_creature);
    // 5/5 creature took 4 damage, should be at 1 toughness (5 - 4)
    assert!(target.is_some(), "Creature should still exist (4 damage to 5 toughness)");
}

/// Test that X-cost spell works when mana is already in pool (no tapping needed).
#[test]
fn test_x_cost_spell_with_mana_in_pool() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils_play = spell_in_hand(&mut state, &registry, "Devil's Play", P0);

    // Add 3 red mana directly to pool
    state
        .get_player_mut(P0)
        .mana_pool
        .add(ManaType::Red, 3);

    let _target_creature = ready_creature(&mut state, P1, 2, 2);

    let legal = engine::legal_actions(&state, &registry);
    let cast_action = legal
        .actions
        .iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == devils_play))
        .expect("Devil's Play should be castable with 3R in pool");

    // Should not panic
    let mut new_state = engine::submit_action(&state, cast_action, &registry);

    // Resolve the ChooseXFunding prompt by funding X to the max.
    new_state = resolve_funding_max(&new_state, &registry);

    let stack_objs = new_state.objects_in_zone(Zone::Stack, P0);
    let spell = stack_objs
        .iter()
        .find(|o| o.name == "Devil's Play")
        .expect("Should be on stack");

    // X = 3 total - 1 for {R} = 2
    assert_eq!(spell.x_value, Some(2));
}

/// Test Mikaeus the Lunarch ({X}{W}) with autotap.
#[test]
fn test_mikaeus_x_cost_autotap() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mikaeus = spell_in_hand(&mut state, &registry, "Mikaeus, the Lunarch", P0);

    // 4 Plains: {W} for colored cost + 3 remaining = X=3
    for _ in 0..4 {
        named_permanent(&mut state, &registry, "Plains", P0);
    }

    let legal = engine::legal_actions(&state, &registry);
    let cast_action = legal
        .actions
        .iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == mikaeus))
        .expect("Mikaeus should be castable with 4 Plains");

    // Should not panic
    let _new_state = engine::submit_action(&state, cast_action, &registry);
}

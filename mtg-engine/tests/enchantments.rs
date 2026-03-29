//! Tests for enchantments, auras, and continuous effects.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::types::*;
use mtg_engine::view::GameView;

/// Holy Strength attaches to a creature and gives +1/+2.
#[test]
fn holy_strength_buffs_creature() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs = castable_spell(&mut state, &registry, "Holy Strength", P0);

    // Cast Holy Strength targeting creature.
    state = cast_and_resolve(&state, &registry, hs, vec![Target::Object(creature)]);

    // Aura should be on battlefield, attached to creature.
    assert_eq!(state.get_object(hs).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(hs).unwrap().attached_to, Some(creature));

    // Creature should have effective +1/+2.
    assert_eq!(state.effective_power(creature, &registry), Some(3));
    assert_eq!(state.effective_toughness(creature, &registry), Some(4));
}

/// Aura falls off when enchanted creature dies.
#[test]
fn aura_falls_off_when_creature_dies() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs = castable_spell(&mut state, &registry, "Holy Strength", P0);

    // Attach Holy Strength.
    state = cast_and_resolve(&state, &registry, hs, vec![Target::Object(creature)]);

    // Kill the creature.
    state.move_object(creature, Zone::Graveyard);

    // SBA should put the unattached aura in graveyard.
    check_state_based_actions_with_registry(&mut state, Some(&registry));
    assert_eq!(state.get_object(hs).unwrap().zone, Zone::Graveyard);
}

/// Pacifism prevents a creature from attacking.
#[test]
fn pacifism_prevents_attacking() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    let pac = castable_spell(&mut state, &registry, "Pacifism", P1);
    state.priority_player = Some(P1); // P1 casts it

    // Cast Pacifism on P0's creature.
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: pac, targets: vec![Target::Object(creature)] },
        &registry,
    );
    state.priority_player = Some(P0); // back to P0
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Creature should not be able to attack.
    assert!(!state.can_attack(creature, &registry));

    // And should not be able to block.
    assert!(!state.can_block(creature, &registry));
}

/// Glorious Anthem gives +1/+1 to all your creatures.
#[test]
fn glorious_anthem_buffs_all_creatures() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 2);
    let c2 = ready_creature(&mut state, P0, 1, 1);
    let opp_creature = ready_creature(&mut state, P1, 3, 3);

    // Put Glorious Anthem on battlefield.
    let anthem = castable_spell(&mut state, &registry, "Glorious Anthem", P0);

    state = cast_and_resolve(&state, &registry, anthem, vec![]);
    assert_eq!(state.get_object(anthem).unwrap().zone, Zone::Battlefield);

    // Your creatures should get +1/+1.
    assert_eq!(state.effective_power(c1, &registry), Some(3));
    assert_eq!(state.effective_toughness(c1, &registry), Some(3));
    assert_eq!(state.effective_power(c2, &registry), Some(2));
    assert_eq!(state.effective_toughness(c2, &registry), Some(2));

    // Opponent's creature should NOT get the bonus.
    assert_eq!(state.effective_power(opp_creature, &registry), Some(3));
    assert_eq!(state.effective_toughness(opp_creature, &registry), Some(3));
}

/// Giant Growth gives +3/+3 until end of turn, then wears off.
#[test]
fn giant_growth_until_end_of_turn() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let gg = castable_spell(&mut state, &registry, "Giant Growth", P0);

    // Cast Giant Growth on creature.
    state = cast_and_resolve(&state, &registry, gg, vec![Target::Object(creature)]);

    // Should have +3/+3 (effective 5/5).
    assert_eq!(state.effective_power(creature, &registry), Some(5));
    assert_eq!(state.effective_toughness(creature, &registry), Some(5));

    // Advance to cleanup — effect should wear off.
    loop {
        engine::advance_step(&mut state, &registry);
        if state.step == Step::Cleanup {
            break;
        }
    }

    // Effect should be gone.
    assert_eq!(state.until_end_of_turn_effects.len(), 0);
    assert_eq!(state.effective_power(creature, &registry), Some(2));
    assert_eq!(state.effective_toughness(creature, &registry), Some(2));
}

/// A creature with Holy Strength (+1/+2) survives damage that would
/// kill its base toughness, when checked with the registry.
#[test]
fn aura_toughness_bonus_prevents_death() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // 2/2 creature with Holy Strength = effective 3/4.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs = castable_spell(&mut state, &registry, "Holy Strength", P0);

    state = cast_and_resolve(&state, &registry, hs, vec![Target::Object(creature)]);

    // Deal 3 damage — enough to kill a 2/2 but not a 3/4.
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    check_state_based_actions_with_registry(&mut state, Some(&registry));
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "3/4 creature with 3 damage should survive");

    // Deal 4th damage — now it dies.
    state.get_object_mut(creature).unwrap().damage_marked = 4;
    check_state_based_actions_with_registry(&mut state, Some(&registry));
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "3/4 creature with 4 damage should die");
}

/// GameView should show effective P/T for creatures with auras.
#[test]
fn view_shows_effective_pt_with_aura() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 1);
    let hs = castable_spell(&mut state, &registry, "Holy Strength", P0);

    state = cast_and_resolve(&state, &registry, hs, vec![Target::Object(creature)]);

    let view = GameView::for_player(&state, P0, &registry);
    let perm = view.battlefield.iter().find(|p| p.object_id == creature).unwrap();

    assert_eq!(perm.power, Some(2), "Raw power should be 2");
    assert_eq!(perm.toughness, Some(1), "Raw toughness should be 1");
    assert_eq!(perm.effective_power, Some(3), "Effective power should be 3 with Holy Strength");
    assert_eq!(perm.effective_toughness, Some(3), "Effective toughness should be 3 with Holy Strength");
}

/// GameView should show aura attachment on the PermanentView.
#[test]
fn view_shows_aura_attached_to_creature() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let hs = castable_spell(&mut state, &registry, "Holy Strength", P0);

    state = cast_and_resolve(&state, &registry, hs, vec![Target::Object(creature)]);

    let view = GameView::for_player(&state, P0, &registry);
    let aura = view.battlefield.iter().find(|p| p.object_id == hs).unwrap();

    assert_eq!(aura.attached_to, Some(creature),
        "Aura should show as attached to the creature in the view");
}

/// GameView shows effective P/T with Glorious Anthem.
#[test]
fn view_shows_effective_pt_with_anthem() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    let anthem = castable_spell(&mut state, &registry, "Glorious Anthem", P0);

    state = cast_and_resolve(&state, &registry, anthem, vec![]);

    let view = GameView::for_player(&state, P0, &registry);
    let perm = view.battlefield.iter().find(|p| p.object_id == creature).unwrap();

    assert_eq!(perm.effective_power, Some(3), "Effective power should be 3 with Anthem");
    assert_eq!(perm.effective_toughness, Some(3), "Effective toughness should be 3 with Anthem");
}

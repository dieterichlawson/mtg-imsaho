//! Tests for Olivia Voldaren.
//!
//! Oracle: {2}{B}{R} 3/3 Legendary Vampire, Flying
//! {1}{R}: Deal 1 damage to another target creature, make it a Vampire, +1/+1 counter on Olivia.
//! {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::types::*;
/// Ability 0: Deal 1 damage, make target a Vampire, +1/+1 counter on Olivia.
#[test]
fn olivia_ability_0_deals_damage_and_makes_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target).unwrap().subtypes = vec!["Human".into()];

    // Activate ability 0.
    activate_via_hooks(&mut state, &reg, olivia, 0, &[Target::Object(target)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Target should have 1 damage and be a Vampire now.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 1, "Should deal 1 damage");
    assert!(target_obj.subtypes.contains(&"Vampire".to_string()),
        "Target should become a Vampire");
    assert!(target_obj.subtypes.contains(&"Human".to_string()),
        "Target should retain Human subtype");

    // Olivia should have a +1/+1 counter.
    assert_eq!(counters_of(&state, olivia, CounterType::PlusOnePlusOne), 1,
        "Olivia should have 1 +1/+1 counter");
}

/// Ability 0 can't target Olivia herself ("another").
#[test]
fn olivia_ability_0_cannot_target_self() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);

    activate_via_hooks(&mut state, &reg, olivia, 0, &[Target::Object(olivia)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Olivia should have no +1/+1 counter (ability should be a no-op for self-target).
    assert_eq!(counters_of(&state, olivia, CounterType::PlusOnePlusOne), 0,
        "Olivia should not be able to target herself with ability 0");
}

/// Ability 1: Gain control of target Vampire.
#[test]
fn olivia_ability_1_steals_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(target).unwrap().subtypes = vec!["Vampire".into()];

    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(target)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Target should now be controlled by P0.
    assert_eq!(state.get_object(target).unwrap().controller, P0,
        "Olivia should steal the Vampire");
}

/// Ability 1 should not steal non-Vampire creatures.
#[test]
fn olivia_ability_1_rejects_non_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(target).unwrap().subtypes = vec!["Human".into()];

    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(target)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Target should still be controlled by P1 (not a Vampire).
    assert_eq!(state.get_object(target).unwrap().controller, P1,
        "Olivia should not steal a non-Vampire");
}

/// When Olivia leaves the battlefield, stolen creatures return to their original controller.
#[test]
fn olivia_stolen_creatures_return_when_olivia_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let target1 = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(target1).unwrap().subtypes = vec!["Vampire".into()];
    let target2 = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target2).unwrap().subtypes = vec!["Vampire".into()];

    // Steal both creatures.
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(target1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(target2)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(target1).unwrap().controller, P0);
    assert_eq!(state.get_object(target2).unwrap().controller, P0);

    // Olivia leaves the battlefield. The control effect's condition is
    // checked as a state-based action, not by a handler on the card.
    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    // Both stolen creatures should return to P1.
    assert_eq!(state.get_object(target1).unwrap().controller, P1,
        "Stolen creature should return to original controller when Olivia leaves");
    assert_eq!(state.get_object(target2).unwrap().controller, P1,
        "Stolen creature should return to original controller when Olivia leaves");
}

/// Ability 1 target filter should only allow Vampires.
#[test]
fn olivia_ability_1_target_filter_requires_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(vampire).unwrap().subtypes = vec!["Vampire".into()];
    let human = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];

    let behavior = reg.get(state.get_object(olivia).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, olivia, &reg);

    // Ability 1 should have a Vampire target filter.
    let ability_1 = &abilities[1];
    match &ability_1.target_requirement {
        Some(mtg_engine::cards::TargetRequirement::CreatureWithFilter(
            mtg_engine::cards::TargetFilter::HasSubtype(s)
        )) => {
            assert_eq!(s, "Vampire", "Ability 1 should filter for Vampires");
        }
        other => panic!("Expected HasSubtype(Vampire) filter, got {other:?}"),
    }
}

/// Ruling: "If you activate Olivia Voldaren's last ability, and before that
/// ability resolves you lose control of Olivia Voldaren, the ability will
/// resolve with no effect. You won't gain control of the targeted Vampire."
///
/// "for as long as **you** control Olivia Voldaren" is a duration, and "you"
/// is the player who activated the ability (CR 602.2a) — not whoever holds
/// Olivia by the time it resolves. With the duration already over, the effect
/// never starts (CR 611.2b).
#[test]
fn olivia_steal_does_nothing_if_you_lost_olivia_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    // P0's own Vampire. "Target Vampire" has no controller restriction, and
    // targeting one of your own is what makes this test discriminating: if the
    // ability read Olivia's *current* controller instead of its activator, it
    // would hand P0's Vampire to P1.
    let vampire = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(vampire).unwrap().subtypes = vec!["Vampire".into()];

    // P0 activates the steal, then P1 takes Olivia in response.
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(vampire)]);
    state.change_control(olivia, P1);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(vampire).unwrap().controller, P0,
        "the ability resolves with no effect — in particular it must not hand \
         the Vampire to whoever took Olivia");
}

/// The same ability from the other side: P1 stealing Olivia does not let P1
/// use an ability P0 had already put on the stack.
#[test]
fn olivia_steal_still_works_while_you_keep_her() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let victim = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(victim).unwrap().subtypes = vec!["Vampire".into()];

    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(victim)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().controller, P0,
        "the control-change happens normally when the activator still has her");
}

/// Ruling: "If Olivia Voldaren deals lethal damage to a creature with its
/// first activated ability, that creature will become a Vampire before dying."
///
/// State-based actions do not run in the middle of a resolution (CR 117.5), so
/// the type change lands while the creature is still on the battlefield.
#[test]
fn olivia_makes_a_creature_a_vampire_before_it_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    // A 1/1 — Olivia's single point of damage is lethal.
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().subtypes = vec!["Human".into()];

    activate_via_hooks(&mut state, &reg, olivia, 0, &[Target::Object(victim)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "state-based actions have not run yet — it is dying, not dead");
    assert!(state.has_subtype(victim, "Vampire", &reg),
        "it became a Vampire while it was still there");

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard,
        "and then it dies");
}

/// Olivia has no triggered abilities. "for as long as you control Olivia
/// Voldaren" is a duration on the control effect (CR 611.2b), ended by a
/// state-based action — she used to declare a `LeavesBattlefield` trigger for
/// it, long after the handler that implemented it by hand was removed, so
/// every time she left she put an empty ability on the stack.
#[test]
fn olivia_puts_nothing_on_the_stack_when_she_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let victim = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(victim).unwrap().subtypes = vec!["Vampire".into()];
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(victim)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_object(victim).unwrap().controller, P0, "test precondition");

    state.events.clear();
    state.trigger_event_index = 0;
    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    assert!(state.stack.is_empty(),
        "nothing goes on the stack; got {:?}", state.stack);
    assert_eq!(state.get_object(victim).unwrap().controller, P1,
        "and the Vampire is back with its owner regardless");
}

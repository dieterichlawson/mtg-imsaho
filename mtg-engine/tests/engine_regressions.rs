//! Regressions found by code review of the engine, kept as a check that they
//! stay fixed. A test here is about the engine, not about one card.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::engine;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ════════════════════════════════════════════════════════════════════
// Bug #1: Legend Rule (CR 704.5k)
//
// If a player controls two or more legendary permanents with the same
// name, they choose one to keep and the rest go to the graveyard.
// ════════════════════════════════════════════════════════════════════

/// Two legendary creatures with the same name — one should be removed by SBAs.
/// Uses the registry to check legendary status via CardData.supertypes.
/// Since no legendary cards exist in the registry yet, we simulate by
/// setting the `is_legendary` flag directly on the `GameObject`.
#[test]
fn legend_rule_removes_duplicate() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create two legendary creatures with the same name.
    let card_id = CardId(200);
    let legend1 = state.create_object(card_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend1).unwrap().name = "Thalia".into();
    state.get_object_mut(legend1).unwrap().summoning_sick = false;
    state.get_object_mut(legend1).unwrap().is_legendary = true;

    let legend2 = state.create_object(card_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend2).unwrap().name = "Thalia".into();
    state.get_object_mut(legend2).unwrap().summoning_sick = false;
    state.get_object_mut(legend2).unwrap().is_legendary = true;

    check_state_based_actions(&mut state, &reg);

    // SBA should have set up a legend-rule choice.
    assert!(state.awaiting_action.is_some(),
        "Legend rule SBA should present a choice for which legendary to keep");

    // Resolve the choice: keep legend1.
    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(legend1))),
        },
        &reg,
    );

    // Track the two by id. Identifying them by the hand-set name does not
    // work: CR 400.7 restores an object's PRINTED name when it leaves the
    // battlefield, so the loser reverts to whatever `card_id` actually names.
    assert_eq!(new_state.get_object(legend1).unwrap().zone, Zone::Battlefield,
        "Legend rule: the kept legendary stays (CR 704.5k)");
    assert_eq!(new_state.get_object(legend2).unwrap().zone, Zone::Graveyard,
        "Legend rule: the duplicate goes to the graveyard");
}

/// Legendary permanents with DIFFERENT names are fine — both stay.
#[test]
fn legend_rule_different_names_coexist() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let legend1 = state.create_object(CardId(200), P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend1).unwrap().name = "Thalia".into();
    state.get_object_mut(legend1).unwrap().is_legendary = true;

    let legend2 = state.create_object(CardId(201), P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(legend2).unwrap().name = "Geist".into();
    state.get_object_mut(legend2).unwrap().is_legendary = true;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(legend1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(legend2).unwrap().zone, Zone::Battlefield);
}

/// Different players can each control a legendary with the same name.
#[test]
fn legend_rule_different_controllers_ok() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = CardId(200);
    let legend_p0 = state.create_object(card_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend_p0).unwrap().name = "Thalia".into();
    state.get_object_mut(legend_p0).unwrap().is_legendary = true;

    let legend_p1 = state.create_object(card_id, P1, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend_p1).unwrap().name = "Thalia".into();
    state.get_object_mut(legend_p1).unwrap().is_legendary = true;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(legend_p0).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(legend_p1).unwrap().zone, Zone::Battlefield);
}

/// Non-legendary permanents with the same name are unaffected.
#[test]
fn legend_rule_ignores_non_legendary() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(c1).unwrap().name = "Grizzly Bears".into();
    let c2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(c2).unwrap().name = "Grizzly Bears".into();

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Battlefield);
}

// ════════════════════════════════════════════════════════════════════
// Bug #2: +1/+1 and -1/-1 Counter Annihilation (CR 704.5q)
//
// If a permanent has both +1/+1 and -1/-1 counters, they annihilate
// in pairs as a state-based action.
// ════════════════════════════════════════════════════════════════════

/// Equal numbers of +1/+1 and -1/-1 counters: all removed.
/// CR 704.5q: if a permanent has both +1/+1 and -1/-1 counters, N of each are
/// removed, where N is the smaller of the two counts. Five one-case tests used
/// to walk this; the interesting part is the arithmetic at the boundary, so it
/// reads better as the table it always was.
#[test]
fn plus_and_minus_counters_annihilate_in_pairs() {
    // (base size, start +1/+1, start -1/-1, left +1/+1, left -1/-1)
    // Each creature is big enough to survive its own case, so the counter
    // arithmetic is what is under test rather than the toughness check.
    const CASES: &[(i32, u32, u32, u32, u32)] = &[
        (2, 3, 3, 0, 0),   // equal counts cancel out entirely
        (2, 5, 2, 3, 0),   // more plus: the surplus stays
        (5, 1, 4, 0, 3),   // more minus: 5/5 down to 2/2, still alive
        (2, 3, 0, 3, 0),   // only one kind — nothing to annihilate
    ];
    let reg = registry();
    for &(base, plus, minus, left_plus, left_minus) in CASES {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let creature = ready_creature(&mut state, P0, base, base);
        state.add_counters(creature, CounterType::PlusOnePlusOne, plus);
        state.add_counters(creature, CounterType::MinusOneMinusOne, minus);

        check_state_based_actions(&mut state, &reg);

        assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), left_plus,
            "{plus} +1/+1 and {minus} -1/-1 should leave {left_plus} +1/+1");
        assert_eq!(state.get_counter_count(creature, CounterType::MinusOneMinusOne), left_minus,
            "{plus} +1/+1 and {minus} -1/-1 should leave {left_minus} -1/-1");
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
            "a {base}/{base} survives {left_minus} net -1/-1 counters");
    }
}

/// Annihilation happens before the toughness check, so it can still leave a
/// creature dead: a 1/1 with one +1/+1 and two -1/-1 annihilates down to a
/// single -1/-1 and is a 0/0.
#[test]
fn annihilation_can_still_leave_a_creature_dead() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.add_counters(creature, CounterType::PlusOnePlusOne, 1);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 2);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "one -1/-1 survives the annihilation and makes the 1/1 a 0/0");
}

/// More +1/+1 than -1/-1: some +1/+1 remain.
/// More -1/-1 than +1/+1: some -1/-1 remain (creature may die from reduced toughness).
/// Counter annihilation killing a creature: 1/1 with 2 -1/-1 counters
/// gets annihilated down to 1 -1/-1, making it 0/0 — it dies.
/// No annihilation needed when only one type of counter exists.
// ════════════════════════════════════════════════════════════════════
// Bug #3: Spell Fizzle (CR 608.2b)
//
// If all of a spell's targets are illegal when it tries to resolve,
// the spell is countered by game rules (fizzled). It should NOT call
// on_resolve at all.
// ════════════════════════════════════════════════════════════════════

// ════════════════════════════════════════════════════════════════════
// Bug #4: Combat Step Skipping
//
// When no attackers are declared, the engine jumps straight to
// EndCombat, skipping DeclareBlockers and CombatDamage steps.
// Per rules 507-510, all steps should execute in sequence.
// ════════════════════════════════════════════════════════════════════

/// After declaring zero attackers, the game loop skips to `EndCombat`.
/// This tests the game loop code path (not `submit_action`, which doesn't skip).
/// The bug is in `run_game_loop_inner`'s post-action handler for `DeclareAttackers`.
///
/// We test this by running the game loop with a callback that records what
/// steps the game passes through.
#[test]
fn no_attackers_game_loop_skips_to_end_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::BeginCombat, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    ready_creature(&mut state, P0, 3, 3);

    // Fill libraries so we don't hit empty-library SBA.
    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    let mut action_count = 0;

    engine::run_game_loop(&mut state, &reg, |game_state, _player, legal| {
        action_count += 1;

        // Safety valve: don't run forever.
        if action_count > 50 {
            return Action::Concede;
        }

        // When asked to declare attackers, declare none.
        if legal.combat_prompt.is_some() {
            if game_state.step == Step::DeclareAttackers {
                return Action::DeclareAttackers { attackers: vec![] };
            }
            if game_state.step == Step::DeclareBlockers {
                return Action::DeclareBlockers { assignments: vec![] };
            }
        }

        // Otherwise just pass priority to advance the game.
        Action::PassPriority
    });

    // Check that DeclareBlockers was reached by looking at StepStarted events.
    // Auto-pass may skip asking the player, but the step should still be entered.
    let saw_declare_blockers = state.events.iter().any(|e| {
        matches!(e, GameEvent::StepStarted { step: Step::DeclareBlockers })
    });
    // Also check the game log for the step.
    let log_has_blockers = state.game_log.iter().any(|e| {
        e.message.contains("DeclareBlockers")
    });
    assert!(saw_declare_blockers || log_has_blockers,
        "Game loop should pass through DeclareBlockers even with zero attackers (CR 507-510)");
}

// ════════════════════════════════════════════════════════════════════
// Bug #11: Falkenrath Noble Trigger Scope
//
// Falkenrath Noble's Oracle text: "Whenever this creature or another
// creature dies, target player loses 1 life and you gain 1 life."
// Triggers on ANY creature death including itself.
// ════════════════════════════════════════════════════════════════════

/// Falkenrath Noble SHOULD trigger when an opponent's creature dies.
/// Oracle: "Whenever this creature or another creature dies" — any creature.
#[test]
fn falkenrath_noble_triggers_on_opponent_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P0);

    // P1's creature dies.
    let enemy = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(enemy).unwrap().damage_marked = 2;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    // Noble SHOULD trigger — "another creature dies" includes opponent's creatures.
    assert_eq!(state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should gain 1 life when any creature dies");
    assert_eq!(state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain opponent when any creature dies");
}

/// Falkenrath Noble SHOULD trigger when your own creature dies.
#[test]
fn falkenrath_noble_triggers_on_own_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _noble = named_creature(&mut state, &reg, "Falkenrath Noble", P0);
    let ally = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(ally).unwrap().damage_marked = 2;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should gain 1 life when your creature dies");
    assert_eq!(state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain opponent when your creature dies");
}

/// Falkenrath Noble SHOULD trigger on itself dying.
/// Oracle: "Whenever THIS CREATURE or another creature dies" — includes self.
#[test]
fn falkenrath_noble_triggers_on_self_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let noble = named_creature(&mut state, &reg, "Falkenrath Noble", P0);
    state.get_object_mut(noble).unwrap().damage_marked = 5;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    // Noble SHOULD trigger on its own death ("this creature ... dies").
    assert_eq!(state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should trigger on its own death");
    assert_eq!(state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain opponent on its own death");
}

// ════════════════════════════════════════════════════════════════════
// Bugs #12-14: Card P/T Bugs (verify continuous effects fixed them)
//
// These were reported as missing P/T modifiers but should now work
// via the ContinuousEffect system.
// ════════════════════════════════════════════════════════════════════

/// Bug #12: Spectral Flight should give +2/+2 AND flying.
#[test]
fn spectral_flight_gives_plus_two_and_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sf = castable_spell(&mut state, &reg, "Spectral Flight", P0);

    state = cast_and_resolve(&state, &reg, sf, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(4),
        "Spectral Flight should give +2 power");
    assert_eq!(state.effective_toughness(creature, &reg), Some(4),
        "Spectral Flight should give +2 toughness");
    assert!(state.has_keyword(creature, Keyword::Flying, &reg),
        "Spectral Flight should grant flying");
}

/// Bug #13: Furor of the Bitten should give +2/+2 AND force attack.
#[test]
fn furor_of_the_bitten_gives_plus_two_and_forces_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 1, 1);
    let furor = castable_spell(&mut state, &reg, "Furor of the Bitten", P0);

    state = cast_and_resolve(&state, &reg, furor, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "Furor of the Bitten should give +2 power (1 + 2 = 3)");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Furor of the Bitten should give +2 toughness (1 + 2 = 3)");

    // The creature should be forced to attack.
    assert!(state.has_effect(creature, &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), &reg), "Furor of the Bitten should force creature to attack");
}

/// Bug #14: Bonds of Faith should give +2/+2 to Humans.
#[test]
fn bonds_of_faith_gives_plus_two_to_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elder Cathar is a Human.
    let creature = named_creature(&mut state, &reg, "Elder Cathar", P0);

    let base_power = state.effective_power(creature, &reg).unwrap();
    let base_toughness = state.effective_toughness(creature, &reg).unwrap();

    let bof = castable_spell(&mut state, &reg, "Bonds of Faith", P0);
    state = cast_and_resolve(&state, &reg, bof, vec![Target::Object(creature)]);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.effective_power(creature, &reg), Some(base_power + 2),
        "Bonds of Faith should give +2 power to Human");
    assert_eq!(state.effective_toughness(creature, &reg), Some(base_toughness + 2),
        "Bonds of Faith should give +2 toughness to Human");
    assert!(state.can_attack(creature, &reg),
        "Human with Bonds of Faith should still be able to attack");
}

/// Bonds of Faith on a non-Human should prevent attack/block, NOT give +2/+2.
#[test]
fn bonds_of_faith_locks_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let bof = castable_spell(&mut state, &reg, "Bonds of Faith", P0);
    state = cast_and_resolve(&state, &reg, bof, vec![Target::Object(creature)]);
    triggers::process_triggers(&mut state, &reg);

    // Should NOT get +2/+2.
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "Non-Human should NOT get +2 power from Bonds of Faith");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Non-Human should NOT get +2 toughness from Bonds of Faith");

    // Should be locked down.
    assert!(!state.can_attack(creature, &reg),
        "Non-Human with Bonds of Faith should not be able to attack");
    assert!(!state.can_block(creature, &reg),
        "Non-Human with Bonds of Faith should not be able to block");
}

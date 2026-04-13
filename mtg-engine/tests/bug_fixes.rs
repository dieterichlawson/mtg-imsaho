//! Tests for known bugs identified in the code review.
//!
//! Each test is written to FAIL against the current code, documenting
//! the expected behavior per MTG rules. Once a bug is fixed, the test
//! should pass.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ════════════════════════════════════════════════════════════════════
// Bug #1: Legend Rule (CR 704.5k)
//
// If a player controls two or more legendary permanents with the same
// name, they choose one to keep and the rest go to the graveyard.
// ════════════════════════════════════════════════════════════════════

/// Two legendary creatures with the same name — one should be removed by SBAs.
/// Uses the registry to check legendary status via CardData.supertypes.
/// Since no legendary cards exist in the registry yet, we simulate by
/// setting the `is_legendary` flag directly on the GameObject.
#[test]
#[ignore] // Known bug
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

    // One should be on battlefield, one in graveyard.
    let on_bf: Vec<_> = state.objects.values()
        .filter(|o| o.name == "Thalia" && o.zone == Zone::Battlefield)
        .collect();
    let in_gy: Vec<_> = state.objects.values()
        .filter(|o| o.name == "Thalia" && o.zone == Zone::Graveyard)
        .collect();

    assert_eq!(on_bf.len(), 1,
        "Legend rule: only one legendary permanent with the same name should remain (CR 704.5k)");
    assert_eq!(in_gy.len(), 1,
        "Legend rule: the duplicate should be in the graveyard");
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
#[test]
fn counter_annihilation_equal_counts() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.add_counters(creature, CounterType::PlusOnePlusOne, 3);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 3);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), 0,
        "Counter annihilation: equal +1/+1 and -1/-1 should cancel out (CR 704.5q)");
    assert_eq!(state.get_counter_count(creature, CounterType::MinusOneMinusOne), 0);
}

/// More +1/+1 than -1/-1: some +1/+1 remain.
#[test]
fn counter_annihilation_more_plus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.add_counters(creature, CounterType::PlusOnePlusOne, 5);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 2);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), 3,
        "Counter annihilation: 5 plus - 2 minus = 3 plus remaining");
    assert_eq!(state.get_counter_count(creature, CounterType::MinusOneMinusOne), 0);
}

/// More -1/-1 than +1/+1: some -1/-1 remain (creature may die from reduced toughness).
#[test]
fn counter_annihilation_more_minus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 5, 5);

    state.add_counters(creature, CounterType::PlusOnePlusOne, 1);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 4);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), 0,
        "Counter annihilation: 1 plus - 4 minus = 0 plus, 3 minus remaining");
    assert_eq!(state.get_counter_count(creature, CounterType::MinusOneMinusOne), 3);
    // Creature is 5/5 - 3/-3 = 2/2, should survive.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
}

/// Counter annihilation killing a creature: 1/1 with 2 -1/-1 counters
/// gets annihilated down to 1 -1/-1, making it 0/0 — it dies.
#[test]
fn counter_annihilation_can_kill() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 1, 1);

    state.add_counters(creature, CounterType::PlusOnePlusOne, 1);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 2);

    // Need registry so effective_toughness accounts for counters.
    check_state_based_actions(&mut state, &reg);

    // After annihilation: 0 plus, 1 minus. Creature is 0/0 — dies.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "1/1 creature with net -1/-1 from counters should die");
}

/// No annihilation needed when only one type of counter exists.
#[test]
fn counter_annihilation_noop_single_type() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.add_counters(creature, CounterType::PlusOnePlusOne, 3);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), 3,
        "No annihilation when only +1/+1 counters exist");
}

// ════════════════════════════════════════════════════════════════════
// Bug #3: Spell Fizzle (CR 608.2b)
//
// If all of a spell's targets are illegal when it tries to resolve,
// the spell is countered by game rules (fizzled). It should NOT call
// on_resolve at all.
// ════════════════════════════════════════════════════════════════════

/// Lightning Bolt targeting a creature that dies before resolution:
/// the bolt should be countered by game rules (fizzle), not resolve.
#[test]
fn spell_fizzles_when_single_target_illegal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Object(creature)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Creature dies before bolt resolves.
    state.move_object(creature, Zone::Graveyard, &reg);

    // Clear events so we can check what the resolution generates.
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Bolt should be in graveyard.
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);

    // The spell should NOT have resolved — no damage events.
    let has_damage_event = state.events.iter().any(|e| matches!(e, GameEvent::CombatDamageDealt { .. }));
    assert!(!has_damage_event,
        "Spell with illegal target should fizzle, not deal damage (CR 608.2b)");

    // P1 should still be at 20 life (bolt didn't redirect to player).
    assert_eq!(state.get_player(P1).life, 20);
}

/// Doom Blade targeting a creature that leaves: should fizzle.
#[test]
fn destroy_spell_fizzles_when_target_gone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 5, 5);
    let doom = castable_spell(&mut state, &reg, "Doom Blade", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: doom, targets: vec![Target::Object(creature)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Creature exiled before resolution.
    state.move_object(creature, Zone::Exile, &reg);

    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Doom Blade should have fizzled — went to graveyard without resolving.
    assert_eq!(state.get_object(doom).unwrap().zone, Zone::Graveyard);
    // Creature is still in exile (wasn't destroyed because spell fizzled).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile);
}

/// Counterspell targeting a spell that's already gone from the stack.
#[test]
fn counterspell_fizzles_when_target_already_countered() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts bolt.
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // P1 casts Counterspell targeting bolt.
    state.priority_player = Some(P1);
    let counter = castable_spell(&mut state, &reg, "Counterspell", P1);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: counter, targets: vec![Target::Object(bolt)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Bolt is removed by some other effect before Counterspell resolves.
    state.stack.retain(|e| e.as_spell() != Some(bolt));
    state.move_object(bolt, Zone::Graveyard, &reg);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Counter should be in graveyard (fizzled).
    assert_eq!(state.get_object(counter).unwrap().zone, Zone::Graveyard);
    // P1's life is unchanged (bolt never resolved).
    assert_eq!(state.get_player(P1).life, 20);
}

// ════════════════════════════════════════════════════════════════════
// Bug #4: Combat Step Skipping
//
// When no attackers are declared, the engine jumps straight to
// EndCombat, skipping DeclareBlockers and CombatDamage steps.
// Per rules 507-510, all steps should execute in sequence.
// ════════════════════════════════════════════════════════════════════

/// After declaring zero attackers, the game loop skips to EndCombat.
/// This tests the game loop code path (not submit_action, which doesn't skip).
/// The bug is in run_game_loop_inner's post-action handler for DeclareAttackers.
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
    assert!(state.has_continuous_effect(creature, &|e| {
        match e {
            ContinuousEffect::ForceAttack { scope } => Some(scope),
            _ => None,
        }
    }, &reg), "Furor of the Bitten should force creature to attack");
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

//! "Attacking creatures you control get +X/+0" is a static ability.
//!
//! Instigator Gang / Wildblood Pack had it as an `AnyCreatureAttacks` trigger
//! that pushed an until-end-of-turn P/T modifier onto each attacker as it was
//! declared. A one-shot buff applied at declaration diverges from a static
//! effect in three ways, and all three are reachable:
//!
//! - it outlives the combat, because "until end of turn" is longer than
//!   "while attacking";
//! - a creature put onto the battlefield attacking never had an attack
//!   declared for it, so it never got the buff;
//! - a Gang that arrives after attackers are declared misses every attacker
//!   already in combat.
//!
//! As a static continuous effect scoped to
//! `And([ControlledByYou, Attacking])`, all three fall out for free — the
//! effect is re-read every time power is asked for.

mod common;

use common::*;
use mtg_engine::types::*;


/// The ordinary case, both faces.
#[test]
fn attacking_creatures_you_control_get_the_bonus() {
    let reg = registry();
    for (transformed, bonus, base) in [(false, 1, 2), (true, 3, 5)] {
        let mut state = game_at_step(Step::DeclareAttackers, P0);
        let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
        state.get_object_mut(gang).unwrap().is_transformed = transformed;
        let ally = named_creature(&mut state, &reg, "Walking Corpse", P0);
        let ally_base = state.effective_power(ally, &reg).unwrap();

        assert_eq!(state.effective_power(gang, &reg).unwrap(), base,
            "transformed={transformed}: nothing is attacking yet");

        submit_declare_attackers(&mut state, &[(gang, P1), (ally, P1)], &reg);

        assert_eq!(state.effective_power(gang, &reg).unwrap(), base + bonus,
            "transformed={transformed}: the Gang is attacking, so it buffs itself");
        assert_eq!(state.effective_power(ally, &reg).unwrap(), ally_base + bonus,
            "transformed={transformed}: and every other attacker you control");
    }
}

/// A creature you control that is not attacking gets nothing, and neither
/// does an opponent's attacker.
#[test]
fn only_attacking_creatures_you_control_get_the_bonus() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
    let home = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let enemy = named_creature(&mut state, &reg, "Walking Corpse", P1);
    let home_base = state.effective_power(home, &reg).unwrap();
    let enemy_base = state.effective_power(enemy, &reg).unwrap();

    submit_declare_attackers(&mut state, &[(gang, P1)], &reg);

    assert_eq!(state.effective_power(home, &reg).unwrap(), home_base,
        "a creature that stayed home is not attacking");
    assert_eq!(state.effective_power(enemy, &reg).unwrap(), enemy_base,
        "an opponent's creature is not one you control");
}

/// Divergence 1: the buff ends when the creature stops attacking, not at end
/// of turn. Combat ends and the attacker is back to its printed power — in
/// the same turn.
#[test]
fn the_bonus_ends_with_combat_not_with_the_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
    let ally = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let ally_base = state.effective_power(ally, &reg).unwrap();

    submit_declare_attackers(&mut state, &[(gang, P1), (ally, P1)], &reg);
    assert_eq!(state.effective_power(ally, &reg).unwrap(), ally_base + 1);

    // Combat ends; nothing is attacking any more, but the turn goes on.
    state.combat = None;

    assert_eq!(state.effective_power(ally, &reg).unwrap(), ally_base,
        "the creature is no longer attacking, so the static bonus is gone — an \
         until-end-of-turn buff would still be there in the postcombat main phase");
}

/// Divergence 2: a creature put onto the battlefield attacking never had an
/// attack declared for it, so a trigger that fires on declaration misses it.
/// A static effect asks "is it attacking?", and it is.
#[test]
fn a_creature_put_onto_the_battlefield_attacking_gets_the_bonus() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
    submit_declare_attackers(&mut state, &[(gang, P1)], &reg);

    // Geist of Saint Traft's Angel, Hanweir Militia Captain's tokens — a
    // creature that arrives already in combat, with no AttackersDeclared for it.
    let latecomer = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let base = state.effective_power(latecomer, &reg).unwrap();
    state.combat.as_mut().unwrap().attackers.insert(latecomer, P1);

    assert_eq!(state.effective_power(latecomer, &reg).unwrap(), base + 1,
        "it is an attacking creature you control, however it got there");
}

/// Divergence 3: a Gang that arrives after attackers are declared buffs the
/// creatures already attacking. A trigger on declaration cannot — declaration
/// already happened.
#[test]
fn a_gang_that_arrives_mid_combat_buffs_the_creatures_already_attacking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let ally = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let base = state.effective_power(ally, &reg).unwrap();
    submit_declare_attackers(&mut state, &[(ally, P1)], &reg);
    assert_eq!(state.effective_power(ally, &reg).unwrap(), base,
        "no Gang yet");

    named_creature(&mut state, &reg, "Instigator Gang", P0);

    assert_eq!(state.effective_power(ally, &reg).unwrap(), base + 1,
        "the Gang's static ability applies to attackers that were already \
         declared when it arrived");
}

/// And it stops the moment the Gang leaves.
#[test]
fn the_bonus_stops_when_the_gang_leaves_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let gang = named_creature(&mut state, &reg, "Instigator Gang", P0);
    let ally = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let base = state.effective_power(ally, &reg).unwrap();

    submit_declare_attackers(&mut state, &[(gang, P1), (ally, P1)], &reg);
    assert_eq!(state.effective_power(ally, &reg).unwrap(), base + 1);

    mtg_engine::destruction::try_destroy(&mut state, gang, &reg);

    assert_eq!(state.effective_power(ally, &reg).unwrap(), base,
        "the static ability is gone with its source");
}

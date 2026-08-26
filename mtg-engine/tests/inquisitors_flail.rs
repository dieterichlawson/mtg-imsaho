//! Inquisitor's Flail: a doubling replacement effect that applies to combat
//! damage in both directions.
//!
//! Oracle: {2} Artifact — Equipment. "If equipped creature would deal combat
//! damage, it deals double that damage instead. If another source would deal
//! combat damage to equipped creature, it deals double that damage to
//! equipped creature instead. Equip {2}."
//!
//! Both clauses say *combat* damage (CR 510.1c). Fight is not combat damage —
//! CR 701.12a is a pair of creatures dealing damage equal to their power, not
//! a combat damage step — so neither clause applies to it.

mod common;
use common::*;
use mtg_engine::types::*;

/// A creature with the Flail attached, ready to attack.
fn equipped(state: &mut GameState, reg: &mtg_engine::cards::CardRegistry,
            power: i32, toughness: i32) -> ObjectId {
    let creature = ready_creature(state, P0, power, toughness);
    let flail = named_equipment(state, reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(creature);
    creature
}

/// Damage the equipped creature *deals* in combat is doubled — to a player and
/// to a blocking creature alike — and is not doubled when the Flail is on the
/// battlefield but attached to nothing. The unattached row is the control:
/// "8 damage" alone is also what an engine that doubles unconditionally does.
#[test]
fn the_equipped_creature_deals_double_combat_damage() {
    let reg = registry();

    // Unblocked, to a player.
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let creature = equipped(&mut state, &reg, 4, 4);
    attacks_unblocked(&mut state, creature, P1);
    let before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(before - state.get_player(P1).life, 8,
        "a 4-power attacker with the Flail deals 8 to the player");

    // Blocked, to the blocker.
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = equipped(&mut state, &reg, 2, 2);
    let blocker = ready_creature(&mut state, P1, 5, 5);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 4,
        "a 2-power attacker with the Flail deals 4 to its blocker");

    // Control: a Flail on the battlefield but equipping nothing doubles nothing.
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);
    attacks_unblocked(&mut state, creature, P1);
    let before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(before - state.get_player(P1).life, 3,
        "an unattached Flail doubles nothing");
}

/// The second clause: combat damage dealt *to* the equipped creature by
/// another source is doubled too.
#[test]
fn the_equipped_creature_takes_double_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = equipped(&mut state, &reg, 3, 6);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 4,
        "a 2-power blocker deals 4 to the equipped creature, not 2");
}

/// CR 616.1: several doubling replacements each apply once, so two Flails
/// quadruple rather than triple.
#[test]
fn two_flails_quadruple_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    for _ in 0..2 {
        let flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
        state.get_object_mut(flail).unwrap().attached_to = Some(creature);
    }
    attacks_unblocked(&mut state, creature, P1);

    let before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(before - state.get_player(P1).life, 12,
        "3 power doubled twice is 12, not 9");
}

/// Both clauses say *combat* damage, and fight is not combat damage. Neither
/// the damage the equipped creature deals in a fight nor the damage it takes
/// there is doubled.
#[test]
fn fight_damage_is_not_combat_damage_and_is_not_doubled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = equipped(&mut state, &reg, 3, 9);
    let opponent = ready_creature(&mut state, P1, 5, 5);

    mtg_engine::combat::fight(&mut state, creature, opponent, &reg);

    assert_eq!(state.get_object(opponent).unwrap().damage_marked, 3,
        "the equipped creature's fight damage is its power, undoubled");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 5,
        "and the fight damage it takes is undoubled too");
}

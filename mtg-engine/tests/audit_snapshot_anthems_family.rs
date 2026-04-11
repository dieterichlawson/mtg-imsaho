//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "Snapshot anthems — until-end-of-turn effects
//! baked at resolution" family. The pattern is: an instant says
//! "Creatures you control get +N/+N until end of turn" but the
//! implementation walks the battlefield at resolution time and
//! pushes one per-target `ModifyPT` effect, missing creatures that
//! enter the battlefield after the anthem resolves.
//!
//! Bugs covered in this file:
//! - Bug AP: Rally the Peasants (and similar) snapshots its anthem at
//!   resolution. A creature entering after the anthem resolves doesn't
//!   get +2/+0.
//! - Bug BK: Instigator Gang's static "attacking creatures you control
//!   get +1/+0" is implemented as an AnyCreatureAttacks trigger that
//!   pushes a per-attacker ModifyPT into `until_end_of_turn`. The buff
//!   then persists after Instigator Gang dies, contrary to its static
//!   nature.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Bug AP (audits/AUDIT_BUGS.md): Rally the Peasants' "+2/+0 until end
/// of turn" anthem snapshots the controller's creatures at resolution
/// time. A creature entering the battlefield after Rally resolves
/// doesn't get the buff, contrary to the continuous wording of the
/// oracle.
///
/// Oracle (Rally the Peasants): "Creatures you control get +2/+0 until
/// end of turn. Flashback {2}{R}."
///
/// Failure mode: `rally_the_peasants.rs:30-51` collects creature ids
/// at resolution, then pushes one `TemporaryEffect::ModifyPT` per
/// existing target into `state.until_end_of_turn`. The effective_power
/// machinery only walks `until_end_of_turn` for matches against the
/// queried object's id, so a creature entering after Rally resolved
/// has no matching ModifyPT and doesn't get the +2/+0.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AP is fixed.
#[test]
fn bug_ap_rally_the_peasants_buffs_creatures_entering_later() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Resolve Rally the Peasants directly. No creatures are in play
    // yet, so there are no targets to snapshot at resolution time.
    let rally_card_id = registry.get_id_by_name("Rally the Peasants").unwrap();
    let rally = state.create_object(rally_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(rally).unwrap().name = "Rally the Peasants".into();
    let behavior = registry.get(rally_card_id).unwrap();
    behavior.on_resolve(&mut state, rally, &[], &registry);

    // Now a creature enters the battlefield. Per the continuous
    // wording of the anthem, it should be +2/+0.
    let bears = ready_creature(&mut state, P0, 2, 2);
    state
        .get_object_mut(bears)
        .unwrap()
        .card_types = vec![CardType::Creature];

    let eff_p = state.effective_power(bears, &registry).unwrap_or(0);
    assert_eq!(
        eff_p, 4,
        "A creature entering the battlefield after Rally the Peasants \
         resolved should still get the +2/+0 anthem (the wording is \
         'Creatures you control get +2/+0 until end of turn'). \
         Bug AP: Rally snapshots its targets at resolution time and \
         pushes per-creature ModifyPT effects, so newcomers are missed. \
         Got effective_power = {}, expected 4 (2 base + 2 anthem).",
        eff_p,
    );
}

/// Bug BK (audits/AUDIT_BUGS.md): Instigator Gang's "Attacking
/// creatures you control get +1/+0" is modeled as an
/// `AnyCreatureAttacks` triggered ability that pushes a per-attacker
/// `TemporaryEffect::ModifyPT` into `until_end_of_turn`. Once
/// Instigator Gang dies, the buff stays in `until_end_of_turn` until
/// end of turn — but per the oracle the static ability turns OFF the
/// moment its source leaves the battlefield.
///
/// Oracle (Instigator Gang front face): "Attacking creatures you
/// control get +1/+0."
///
/// Failure mode: `instigator_gang.rs:89-110` runs in
/// `on_any_creature_attacks` and writes a `ModifyPT { target,
/// power_mod: 1, ... }` into `until_end_of_turn`. The effect is keyed
/// to the attacker (not the source Instigator Gang), so the
/// effective_power machinery keeps reporting +1 power for the attacker
/// even after Instigator Gang has been removed from the battlefield.
///
/// We simulate the trigger firing by manually pushing the same
/// `ModifyPT` effect into `until_end_of_turn` (matching what
/// `on_any_creature_attacks` does), then move Instigator Gang to the
/// graveyard. The bug means the attacker still reports the +1
/// power; the fix should make the static ability evaluate live, so
/// the bonus disappears once Instigator Gang is gone.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BK is fixed.
#[test]
fn bug_bk_instigator_gang_anthem_drops_when_source_leaves() {
    use mtg_engine::state::TemporaryEffect;

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let gang = named_creature(&mut state, &registry, "Instigator Gang", P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state
        .get_object_mut(attacker)
        .unwrap()
        .card_types = vec![CardType::Creature];

    // Mirror what instigator_gang.rs's on_any_creature_attacks does:
    // push a ModifyPT into until_end_of_turn keyed to the attacker.
    state.until_end_of_turn.push(TemporaryEffect::ModifyPT {
        target: attacker,
        power_mod: 1,
        toughness_mod: 0,
    });
    let buffed_p = state.effective_power(attacker, &registry).unwrap_or(0);
    assert_eq!(
        buffed_p, 3,
        "Test setup: with Instigator Gang in play and the trigger fired, \
         the attacker should be 3/2"
    );

    // Now Instigator Gang dies (mid-combat removal, e.g. Brimstone Volley
    // resolved on it before the damage step). Per oracle, the static
    // ability turns off the moment Instigator Gang leaves the battlefield.
    state.move_object(gang, Zone::Graveyard, &registry);

    let after_p = state.effective_power(attacker, &registry).unwrap_or(0);
    assert_eq!(
        after_p, 2,
        "Once Instigator Gang dies, the attacker should drop back to its \
         base 2/2 — the +1/+0 was a static ability that should turn off \
         when its source leaves the battlefield. Bug BK: the buff is \
         baked into until_end_of_turn at trigger time and stays there \
         past Instigator Gang's death. Got effective_power = {}, \
         expected 2.",
        after_p,
    );
}

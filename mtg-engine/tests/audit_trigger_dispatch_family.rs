//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers a slice of the "Trigger dispatch and timing"
//! family — bugs whose root cause is in card-level handlers reading
//! the wrong source-state field, or in the dispatcher reaching the
//! wrong handler.
//!
//! Bugs covered in this file:
//! - Bug BT: `on_any_creature_dies` handlers zone-gate on Battlefield,
//!   silently dropping triggers when the watcher dies simultaneously
//!   with its target (Abattoir Ghoul mutual first-strike trade)
//! - Bug L: Charmbreaker Devils' SpellCast trigger fires for every
//!   spell type instead of only instants/sorceries
//! - Bug CA: Moldgraf Monstrosity reads `o.owner` instead of
//!   `o.controller`, returning creatures from the wrong player's
//!   graveyard when stolen

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Bug BT (audits/AUDIT_BUGS.md): Abattoir Ghoul's
/// `on_any_creature_dies` handler early-returns when its self_id is
/// not on the battlefield. In a mutual first-strike trade where the
/// Ghoul and a creature it dealt damage to die simultaneously, the
/// trigger queue still picks up the death (the dispatcher correctly
/// includes simultaneously-dead watchers), but the handler then drops
/// the effect because it sees the Ghoul is in graveyard.
///
/// Oracle (Abattoir Ghoul): "First strike. Whenever a creature dealt
/// damage by this creature this turn dies, you gain life equal to
/// that creature's toughness."
///
/// Failure mode: `abattoir_ghoul.rs:39-42` does
/// ```
/// let controller = match state.get_object(self_id) {
///     Some(o) if o.zone == Zone::Battlefield => o.controller,
///     _ => return,
/// };
/// ```
/// CR 603.6d / 603.10c says a triggered ability that has been put on
/// the stack continues to resolve even if the source has left the
/// battlefield. Falkenrath Noble's death-trigger handler is the
/// counter-example that gets this right.
///
/// We simulate the audit-confirmed scenario: Voiceless Spirit
/// (toughness 1) was damaged by Abattoir Ghoul, then both die
/// simultaneously. Calling the dispatcher's
/// `on_any_creature_dies` directly with the captured `damaged_by` and
/// `dead_toughness` mirrors what `triggers.rs` does at trigger
/// resolution time.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BT is fixed.
#[test]
fn bug_bt_abattoir_ghoul_gains_life_on_simultaneous_death() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Abattoir Ghoul belongs to P0, who should gain 1 life from the
    // dying Voiceless Spirit (1 toughness).
    let ghoul = named_creature(&mut state, &registry, "Abattoir Ghoul", P0);
    // Move the Ghoul to graveyard to mirror the simultaneous-death
    // state at trigger-resolution time.
    state.move_object(ghoul, Zone::Graveyard, &registry);

    let life_before = state.get_player(P0).life;

    // Fire the AnyCreatureDies handler. The dispatcher uses the
    // captured `damaged_by` and `dead_toughness`; we hand-craft them
    // to match what the trigger collector would record.
    let ghoul_card_id = registry.get_id_by_name("Abattoir Ghoul").unwrap();
    let dummy_dead = mtg_engine::ids::ObjectId(99999);
    let dead_damaged_by = vec![ghoul];
    let dead_toughness = 1;
    let behavior = registry.get(ghoul_card_id).unwrap();
    behavior.on_any_creature_dies(
        &mut state,
        ghoul,
        dummy_dead,
        P1,
        &dead_damaged_by,
        dead_toughness,
        &registry,
    );

    let life_after = state.get_player(P0).life;
    assert_eq!(
        life_after - life_before,
        1,
        "Abattoir Ghoul should gain 1 life from a simultaneously-dying \
         creature it dealt damage to (CR 603.6d: triggered ability \
         continues to resolve even if its source has left the \
         battlefield). Bug BT: the handler early-returns because the \
         Ghoul is no longer on the battlefield. Life: {} -> {}",
        life_before, life_after,
    );
}

/// Bug L (audits/AUDIT_BUGS.md): Charmbreaker Devils' `on_spell_cast`
/// triggered ability fires for every spell type, not just
/// instants/sorceries.
///
/// Oracle (Charmbreaker Devils): "Whenever you cast an instant or
/// sorcery spell, this creature gets +4/+0 until end of turn."
///
/// Failure mode: `charmbreaker_devils.rs:75-92` filters by `caster ==
/// controller` but does NOT filter by spell type. The dispatcher
/// (`triggers.rs:727`) explicitly says "Dispatch SpellCast triggers
/// for ALL spell types... Individual card handlers can filter by
/// spell type if needed" — Charmbreaker doesn't.
///
/// We exercise the bug by calling Charmbreaker's `on_spell_cast`
/// handler directly with a creature spell as the trigger source. The
/// handler should be a no-op (Grizzly Bears is a creature, not an
/// instant or sorcery). Today the handler unconditionally pushes a
/// `+4/+0` ModifyPT into `until_end_of_turn`.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug L is fixed.
#[test]
fn bug_l_charmbreaker_devils_does_not_buff_on_creature_spell() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_creature(&mut state, &registry, "Charmbreaker Devils", P0);
    let base_power = state.effective_power(devils, &registry).unwrap_or(0);

    // Spawn a Grizzly Bears spell on the stack and dispatch the
    // SpellCast trigger to Charmbreaker manually.
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let bears_spell = state.create_object(bears_card_id, P0, Zone::Stack, Some(2), Some(2));
    state.get_object_mut(bears_spell).unwrap().name = "Grizzly Bears".into();

    let devils_card_id = registry.get_id_by_name("Charmbreaker Devils").unwrap();
    let behavior = registry.get(devils_card_id).unwrap();
    behavior.on_spell_cast(&mut state, devils, P0, bears_spell, &registry);

    let after_power = state.effective_power(devils, &registry).unwrap_or(0);
    assert_eq!(
        after_power, base_power,
        "Charmbreaker Devils' +4/+0 should NOT trigger when the \
         controller casts a creature spell — its oracle text restricts \
         the trigger to instants and sorceries. Bug L: the handler \
         doesn't filter by spell type. effective_power: {} -> {}",
        base_power, after_power,
    );
}

/// Bug CA (audits/AUDIT_BUGS.md): Moldgraf Monstrosity reads
/// `o.owner` instead of `o.controller` for its "your graveyard"
/// reference, so when stolen via Traitorous Blood and dying that
/// turn, it returns creatures from the WRONG player's graveyard.
///
/// Oracle (Moldgraf Monstrosity): "When this creature dies, exile it,
/// then return two creature cards at random from **your** graveyard
/// to the battlefield."
///
/// CR 603.10c: "If a permanent leaves the battlefield, the owner's
/// controller and other characteristics for the duration of leaving
/// triggers are set from last known information just before that
/// event." So "your" should be the last-known *controller*, which is
/// the Traitorous Blood caster — not the original owner.
///
/// Failure mode: `moldgraf_monstrosity.rs:42-46` reads `o.owner`,
/// while Doomed Traveler and Mausoleum Guard correctly read
/// `o.controller`. We test by giving Moldgraf an `owner != controller`
/// state and observing whose graveyard is reanimated from.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug CA is fixed.
#[test]
fn bug_ca_moldgraf_monstrosity_uses_controller_not_owner() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    // Moldgraf is owned by P1 but currently controlled by P0
    // (modeling a Traitorous Blood theft).
    let mold_card_id = registry.get_id_by_name("Moldgraf Monstrosity").unwrap();
    let mold = state.create_object(mold_card_id, P1, Zone::Battlefield, Some(8), Some(8));
    {
        let obj = state.get_object_mut(mold).unwrap();
        obj.name = "Moldgraf Monstrosity".into();
        obj.controller = P0; // stolen
    }

    // P0 (the new controller / "you") has a creature card in their
    // graveyard. P1 (the original owner) has none. The fix should
    // reanimate from P0's graveyard; the bug reanimates from P1's.
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let p0_bears = state.create_object(bears_card_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(p0_bears).unwrap().name = "Grizzly Bears (P0)".into();

    // Fire Moldgraf's death trigger directly.
    let behavior = registry.get(mold_card_id).unwrap();
    behavior.on_dies(&mut state, mold, &registry);

    let bears_zone = state.get_object(p0_bears).map(|o| o.zone);
    assert_eq!(
        bears_zone,
        Some(Zone::Battlefield),
        "Moldgraf Monstrosity (controlled by P0 via theft) should return \
         creatures from P0's graveyard, not from P1's owner-graveyard. \
         Bug CA: the handler reads o.owner (P1, who has nothing in \
         graveyard) instead of o.controller (P0). Grizzly Bears zone: {:?}",
        bears_zone,
    );
}

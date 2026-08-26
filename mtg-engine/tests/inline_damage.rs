//! Damage from a card effect goes through the engine's pipeline, so the
//! checks a permanent is entitled to still happen: protection from the source
//! (CR 702.16e), "prevent that damage, remove a +1/+1 counter" (CR 614.1a),
//! and loyalty removal rather than marked damage on a planeswalker
//! (CR 120.3c).
//!
//! These used to be eleven tests, one per card that had once written
//! `damage_marked` itself, each calling that card's hook directly. Calling a
//! hook cannot tell you whether the *engine* honours these rules, and naming
//! eleven cards says nothing about the two hundred others. The invariant is a
//! source guard now — `test_suite_guards.rs::only_the_damage_pipeline_marks_damage`
//! fails the build for any card that marks damage itself — and what is left
//! here is the pipeline's own behaviour, plus the cards whose damage takes an
//! unusual route to it.

mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::damage::{deal_damage, DamageKind};
use mtg_engine::events::DamageTarget;
use mtg_engine::ids::CardId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

fn make_planeswalker(state: &mut GameState, owner: PlayerId, loyalty: u32) -> ObjectId {
    let id = state.create_object(CardId(9998), owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = "Test Planeswalker".into();
    obj.card_types = vec![CardType::Planeswalker];
    obj.counters.insert(CounterType::Loyalty, loyalty);
    obj.summoning_sick = false;
    id
}

/// Give `id` protection from a subtype, and give the source that subtype, so
/// the two match (CR 702.16e).
fn protect_from_subtype(state: &mut GameState, id: ObjectId, subtype: &str) {
    state.get_object_mut(id).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: subtype.into(),
            scope: EffectScope::OnSelf,
        },
    ]);
}

// ---------------------------------------------------------------------------
// What the pipeline does that a hand-rolled `damage_marked +=` would not
// ---------------------------------------------------------------------------

/// Protection from the source prevents the damage, whether it is combat damage
/// or a card's effect. The unprotected creature beside it is the control: with
/// only the protected one, a pipeline that dealt no damage at all would pass.
#[test]
fn protection_from_the_source_prevents_its_damage() {
    for kind in [DamageKind::Combat, DamageKind::NonCombat] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let source = ready_creature(&mut state, P0, 3, 3);
        state.get_object_mut(source).unwrap().subtypes.push("Dragon".into());

        let protected = ready_creature(&mut state, P1, 3, 15);
        protect_from_subtype(&mut state, protected, "Dragon");
        let exposed = ready_creature(&mut state, P1, 3, 15);

        deal_damage(&mut state, source, DamageTarget::Object(protected), 6, kind, &reg);
        deal_damage(&mut state, source, DamageTarget::Object(exposed), 6, kind, &reg);

        assert_eq!(state.get_object(protected).unwrap().damage_marked, 0,
            "{kind:?}: protection from Dragons prevents the Dragon's damage");
        assert_eq!(state.get_object(exposed).unwrap().damage_marked, 6,
            "{kind:?}: and the creature without it takes the full amount");
    }
}

/// "If damage would be dealt to Unbreathing Horde, prevent that damage and
/// remove a +1/+1 counter from it instead" (CR 614.1a) — a replacement effect,
/// so it applies however the damage arrives.
#[test]
fn prevent_and_remove_a_counter_replaces_the_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let source = ready_creature(&mut state, P0, 13, 13);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);
    let vanilla = ready_creature(&mut state, P1, 3, 15);

    deal_damage(&mut state, source, DamageTarget::Object(horde), 13, DamageKind::NonCombat, &reg);
    deal_damage(&mut state, source, DamageTarget::Object(vanilla), 13, DamageKind::NonCombat, &reg);

    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0, "the damage is prevented");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 2,
        "and one +1/+1 counter is removed instead");
    assert_eq!(state.get_object(vanilla).unwrap().damage_marked, 13,
        "the creature without the replacement takes all 13");
}

/// CR 120.3c: damage to a planeswalker removes that many loyalty counters. It
/// does not become marked damage, which nothing would ever clear.
#[test]
fn damage_to_a_planeswalker_removes_loyalty_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let source = ready_creature(&mut state, P0, 3, 3);
    let pw = make_planeswalker(&mut state, P1, 4);

    deal_damage(&mut state, source, DamageTarget::Object(pw), 3, DamageKind::NonCombat, &reg);

    assert_eq!(counters_of(&state, pw, CounterType::Loyalty), 1,
        "3 damage to a 4-loyalty planeswalker leaves 1");
    assert_eq!(state.get_object(pw).unwrap().damage_marked, 0,
        "and none of it is marked on the permanent");
}

/// A planeswalker is a permanent like any other, so protection and prevention
/// reach it too — the loyalty route must not skip them.
#[test]
fn a_planeswalker_keeps_its_loyalty_when_the_damage_is_prevented() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let source = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(source).unwrap().subtypes.push("Bloodhall".into());

    let protected = make_planeswalker(&mut state, P1, 4);
    protect_from_subtype(&mut state, protected, "Bloodhall");

    let replacing = make_planeswalker(&mut state, P1, 4);
    state.get_object_mut(replacing).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventDamageRemoveCounter { scope: EffectScope::OnSelf },
    ]);
    state.add_counters(replacing, CounterType::PlusOnePlusOne, 2);

    deal_damage(&mut state, source, DamageTarget::Object(protected), 2, DamageKind::NonCombat, &reg);
    deal_damage(&mut state, source, DamageTarget::Object(replacing), 2, DamageKind::NonCombat, &reg);

    assert_eq!(counters_of(&state, protected, CounterType::Loyalty), 4,
        "CR 702.16e: protection prevents the damage, so no loyalty is lost");
    assert_eq!(counters_of(&state, replacing, CounterType::Loyalty), 4,
        "CR 614.1a: the replacement prevents it too");
    assert_eq!(counters_of(&state, replacing, CounterType::PlusOnePlusOne), 1,
        "and consumes a +1/+1 counter doing so");
}

// ---------------------------------------------------------------------------
// Cards whose damage reaches the pipeline by an unusual route
// ---------------------------------------------------------------------------

/// Blasphemous Act damages every creature at once rather than a chosen target,
/// so each creature's own protection and replacement have to be consulted
/// per-creature rather than once for the spell.
#[test]
fn blasphemous_act_consults_each_creature_separately() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);
    let vanilla = ready_creature(&mut state, P1, 3, 15);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 8);
    let spell = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "the Horde's replacement applies to its share of the damage");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 2,
        "consuming one of its counters");
    assert_eq!(state.get_object(vanilla).unwrap().damage_marked, 13,
        "and the creature beside it takes the full 13");
}

/// Harvest Pyre's amount is chosen while casting (exile X cards from your
/// graveyard), so the damage is dealt with a value the spell carries rather
/// than a constant — the pipeline still gets to replace it.
#[test]
fn harvest_pyres_chosen_x_still_goes_through_the_pipeline() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);
    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    }

    let pyre = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let state = cast_and_resolve(&state, &reg, pyre, vec![Target::Object(horde)]);

    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "CR 614.1a: the Horde's replacement applies to Harvest Pyre's X as well");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 2,
        "one counter removed instead of the damage");
}

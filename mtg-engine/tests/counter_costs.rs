//! Removing counters is a cost, and a cost is paid before the ability does
//! anything.
//!
//! Grimoire of the Dead is "{T}, Remove three study counters from Grimoire of
//! the Dead and sacrifice it: ...". Removing three counters and sacrificing
//! the permanent are two separate cost actions in a fixed order (CR 601.2h);
//! the sacrifice moves it to the graveyard and clears every counter it has in
//! one step. Leaving the removal to the sacrifice meant a Grimoire with four
//! counters lost all four without three of them ever having been "removed",
//! and nothing that watches counter removal could see it happen.
//!
//! `ActivatedAbilityDef::counter_cost` puts the check and the payment in the
//! engine, so the card no longer hand-rolls "do I have at least three?".

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn grimoire_with(counters: u32) -> (mtg_engine::state::GameState, mtg_engine::ids::ObjectId, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let grimoire = named_creature(&mut state, &reg, "Grimoire of the Dead", P0);
    state.get_object_mut(grimoire).unwrap().power = None;
    state.get_object_mut(grimoire).unwrap().toughness = None;
    state.add_counters(grimoire, CounterType::Study, counters);
    (state, grimoire, reg)
}

fn reanimate_offered(
    state: &mtg_engine::state::GameState,
    reg: &CardRegistry,
    grimoire: mtg_engine::ids::ObjectId,
) -> bool {
    mtg_engine::engine::legal_actions(state, reg).actions.iter().any(|a|
        matches!(a, Action::ActivateAbility { object_id, ability_index: 1, .. } if *object_id == grimoire))
}

/// Fewer than three counters: the cost can't be paid, so the ability isn't
/// offered. This used to be a hand-rolled check inside the card.
#[test]
fn the_ability_is_not_offered_without_enough_counters() {
    let reg = registry();
    for counters in 0..3u32 {
        let (state, grimoire, _) = grimoire_with(counters);
        assert!(!reanimate_offered(&state, &reg, grimoire),
            "{counters} study counters is not three");
    }
    let (state, grimoire, _) = grimoire_with(3);
    assert!(reanimate_offered(&state, &reg, grimoire),
        "three study counters pays the cost");
}

/// Exactly three counters are removed, not "all of them". With four on the
/// permanent the fourth is still there when the sacrifice takes it, rather
/// than being swallowed by the same step.
#[test]
fn only_three_counters_removed_when_four_present() {
    let (mut state, grimoire, reg) = grimoire_with(4);
    let corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    // Pay the cost by hand in the order the engine does, so the intermediate
    // state is observable: counters first, then the sacrifice.
    state.remove_counters(grimoire, CounterType::Study, 3);
    assert_eq!(counters_of(&state, grimoire, CounterType::Study), 1,
        "three counters removed, the fourth still on the permanent");

    mtg_engine::destruction::sacrifice(&mut state, grimoire, &reg);
    assert_eq!(counters_of(&state, grimoire, CounterType::Study), 0,
        "the zone change clears what is left");
    assert_eq!(state.get_object(corpse).unwrap().zone, Zone::Graveyard,
        "test setup only — the effect itself is exercised below");
}

/// End to end through the engine: activating removes three and sacrifices,
/// and the creature cards come back.
#[test]
fn activating_pays_the_counter_cost_then_sacrifices() {
    let (mut state, grimoire, reg) = grimoire_with(4);
    let corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let action = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, ability_index: 1, .. } if *object_id == grimoire))
        .expect("the ability should be offered with four counters");
    state = mtg_engine::engine::submit_action(&state, &action, &reg);

    assert_eq!(state.get_object(grimoire).unwrap().zone, Zone::Graveyard,
        "the Grimoire sacrificed itself");
    assert_eq!(state.get_object(corpse).unwrap().zone, Zone::Battlefield,
        "and every creature card in every graveyard came back");
}

/// `remove_counters` takes what is there and no more.
#[test]
fn removing_more_counters_than_present_does_not_underflow() {
    let (mut state, grimoire, _reg) = grimoire_with(2);
    state.remove_counters(grimoire, CounterType::Study, 5);
    assert_eq!(counters_of(&state, grimoire, CounterType::Study), 0);
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug 76-002 (`audits/AUDIT_BUGS.md)`: Ludevic's Test Subject stores
/// its hatchling counters in `obj.card_state` (abused as an `ObjectId`)
/// instead of using `state.add_counters`. Per CR 122 these are real
/// counters; proliferate (CR 701.24) and counter-removal effects
/// can't see them in the abused-card-state form.
///
/// Oracle (Ludevic's Test Subject): "{1}{U}: Put a hatchling counter
/// on this creature. Then if there are five or more hatchling
/// counters on it, remove all of them and transform it."
///
/// Failure mode: `ludevics_test_subject.rs:90-108` does
/// ```
/// obj.card_state.insert("hatchling_counters".into(), ObjectId(new_count as u64));
/// ```
/// instead of using the real counter pipeline (`state.add_counters` /
/// `state.get_counter_count`). Mikaeus the Lunarch in the same set
/// stores +1/+1 counters correctly via `CounterType::PlusOnePlusOne`
/// — that's the model to follow.
///
/// We assert the bug-fingerprint: after activating the hatchling
/// ability, `obj.card_state` should NOT contain a `hatchling_counters`
/// key — the counter must live in the real counter pipeline so other
/// effects can interact with it.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 76-002 is fixed.
#[test]
fn bug_76_002_ludevic_hatchling_counters_not_in_card_state() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ludevic = named_creature(&mut state, &registry, "Ludevic's Test Subject", P0);

    let ludevic_card_id = state.get_object(ludevic).unwrap().card_id;
    let behavior = registry.get(ludevic_card_id).unwrap();
    behavior.on_activate_ability(&mut state, ludevic, 0, &[], &registry);

    let obj = state.get_object(ludevic).unwrap();
    assert!(
        !obj.card_state.contains_key("hatchling_counters"),
        "Ludevic's Test Subject should not store hatchling counters in \
         obj.card_state — they're real CR 122 counters and need to live \
         in the engine's counter pipeline so proliferate and counter \
         removal can interact with them. Bug 76-002: card_state still \
         contains 'hatchling_counters' after activation. card_state = {:?}",
        obj.card_state,
    );
}

//! Regression tests for protection-from-source during *activated ability*
//! targeting (CR 702.16b).
//!
//! `generate_ability_targets` used to call a `can_be_targeted` wrapper that
//! always passed `None` for `source_id`, so the protection check inside
//! `can_be_targeted_by` was silently skipped. Only the spell path
//! (`valid_targets_for_req`) threaded the source through. A creature with
//! protection from the ability's source therefore showed up as a legal
//! target for every activated ability.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::TemporaryEffect;
use mtg_engine::types::*;
/// Collect every target offered for an activated ability of `source`.
fn ability_targets(state: &mtg_engine::state::GameState, reg: &CardRegistry, source: ObjectId) -> Vec<Target> {
    engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { object_id, targets, .. } if *object_id == source => {
                Some(targets.clone())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

/// Give `target` protection from Humans (the Spare from Evil shape).
fn protect_from_humans(state: &mut mtg_engine::state::GameState, target: ObjectId) {
    state.until_end_of_turn.push(TemporaryEffect::GrantProtection {
        target,
        filter: CreatureFilter::HasSubtype("Human".into()),
    });
}

/// Avacynian Priest ({1}, {T}: Tap target non-Human creature) is a Human
/// Cleric, so a non-Human creature with protection from Humans is an illegal
/// target for its ability. `TargetRequirement::CreatureWithFilter`.
#[test]
fn avacynian_priest_cannot_target_creature_with_protection_from_its_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_creature(&mut state, &reg, "Avacynian Priest", P0);
    let zombie = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(zombie).unwrap().subtypes = vec!["Zombie".into()];
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let before = ability_targets(&state, &reg, priest);
    assert!(before.contains(&Target::Object(zombie)),
        "test precondition: an unprotected non-Human creature is a legal target; got {before:?}");

    protect_from_humans(&mut state, zombie);

    let after = ability_targets(&state, &reg, priest);
    assert!(!after.contains(&Target::Object(zombie)),
        "a creature with protection from Humans must not be targetable by the \
         Human Avacynian Priest's ability; got {after:?}");
}

/// `TargetRequirement::Creature` takes the same path. Elder of Laurels
/// ({3}{G}, {T}: Target creature gets +X/+X ...) is a Human Shaman.
#[test]
fn elder_of_laurels_cannot_target_creature_with_protection_from_its_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_creature(&mut state, &reg, "Elder of Laurels", P0);
    let bear = ready_creature(&mut state, P0, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let before = ability_targets(&state, &reg, elder);
    assert!(before.contains(&Target::Object(bear)),
        "test precondition: an unprotected creature is a legal target; got {before:?}");

    protect_from_humans(&mut state, bear);

    let after = ability_targets(&state, &reg, elder);
    assert!(!after.contains(&Target::Object(bear)),
        "a creature with protection from Humans must not be targetable by the \
         Human Elder of Laurels' ability; got {after:?}");
}

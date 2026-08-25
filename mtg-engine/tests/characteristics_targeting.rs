//! Regression tests for characteristics-layer targeting fixes.
//!
//! Non-token permanents have empty object-level `card_types`, so any filter
//! that read `obj.card_types` directly silently excluded them:
//! - `HasCardType([Land])` (Ghost Quarter) found zero non-token lands.
//! - `AnyTarget` (Lightning Bolt) could not target non-token planeswalkers.
//!
//! Both now resolve card types through `GameState::has_card_type`, which
//! falls back to the active face's registry data.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::types::*;
/// Ghost Quarter's "Destroy target land" uses `PermanentWithFilter(HasCardType([Land]))`.
/// A non-token land (empty object-level `card_types`) must be a valid target.
#[test]
fn ghost_quarter_can_target_non_token_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gq = named_creature(&mut state, &reg, "Ghost Quarter", P0);
    let forest = named_creature(&mut state, &reg, "Forest", P1);
    assert!(state.get_object(forest).unwrap().card_types.is_empty(),
        "test precondition: non-token permanents have empty object-level card_types");

    let legal = engine::legal_actions(&state, &reg);
    let gq_targets: Vec<Target> = legal.actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { object_id, targets, .. } if *object_id == gq => {
                Some(targets.clone())
            }
            _ => None,
        })
        .flatten()
        .collect();

    assert!(gq_targets.contains(&Target::Object(forest)),
        "Ghost Quarter should be able to target a non-token land; got targets {gq_targets:?}");
}

/// Lightning Bolt's `AnyTarget` must include non-token planeswalkers.
#[test]
fn any_target_includes_non_token_planeswalker() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P1);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    assert!(state.get_object(liliana).unwrap().card_types.is_empty(),
        "test precondition: non-token permanents have empty object-level card_types");

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    let legal = engine::legal_actions(&state, &reg);
    let bolt_targets: Vec<Target> = legal.actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == bolt => {
                Some(targets.clone())
            }
            _ => None,
        })
        .flatten()
        .collect();

    assert!(bolt_targets.contains(&Target::Object(liliana)),
        "Lightning Bolt (any target) should be able to target a non-token planeswalker; got {bolt_targets:?}");
}

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
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Ghost Quarter's "Destroy target land" uses `PermanentWithFilter(HasCardType([Land]))`.
/// A non-token land (empty object-level `card_types`) must be a valid target.
#[test]
fn ghost_quarter_can_target_non_token_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gq = named_permanent(&mut state, &reg, "Ghost Quarter", P0);
    let forest = named_permanent(&mut state, &reg, "Forest", P1);
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

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
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

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Victim of Night can target Vampire tokens.
/// Oracle: "Destroy target non-Vampire, non-Werewolf, non-Zombie creature."
/// The `is_valid_target` check uses `registry.card_data()` which returns None for
/// tokens, so the subtype exclusion fails and tokens are targetable.
#[test]
fn bug_victim_of_night_can_target_vampire_token() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a Vampire token (like Bloodline Keeper creates)
    let vampire_token = state.create_token_with_subtypes(
        "Vampire", P1, 2, 2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        vec!["Vampire".into()],
        &registry,
    )[0];
    if let Some(obj) = state.get_object_mut(vampire_token) {
        obj.summoning_sick = false;
    }

    // Verify token has Vampire subtype
    assert!(state.get_object(vampire_token).unwrap().subtypes.contains(&"Vampire".into()),
        "Token should have Vampire subtype");

    // Cast Victim of Night targeting the Vampire token
    let _victim = castable_spell(&mut state, &registry, "Victim of Night", P0);

    // Check if the Vampire token is a valid target
    let behavior = registry.get(
        registry.get_id_by_name("Victim of Night").unwrap()
    ).unwrap();
    let is_valid = behavior.is_valid_target(
        &state, P0, &Target::Object(vampire_token), &registry
    );

    // BUG: Token should NOT be a valid target (it's a Vampire),
    // but is_valid_target only checks registry which has no data for tokens
    assert!(!is_valid,
        "Vampire token should NOT be a valid target for Victim of Night");
}

/// Bug: Tribute to Hunger says "target opponent" but has no `is_valid_target`
/// override, so it can target any player including self.
#[test]
fn bug_tribute_to_hunger_can_target_self() {
    let registry = CardRegistry::with_all_cards();
    let state = game_at_step(Step::PrecombatMain, P0);

    // Check if Tribute to Hunger's is_valid_target allows targeting self
    let behavior = registry.get(
        registry.get_id_by_name("Tribute to Hunger").unwrap()
    ).unwrap();

    let can_target_self = behavior.is_valid_target(
        &state, P0, &Target::Player(P0), &registry
    );

    // BUG: "target opponent" should not allow targeting self
    assert!(!can_target_self,
        "Tribute to Hunger says 'target opponent' but allows targeting self");
}

/// Bug: Unburial Rites has no `target_requirement` override, so the engine
/// treats it as an untargeted spell. It can be cast with no creatures
/// in any graveyard, and targets are selected at resolution not cast.
#[test]
fn bug_unburial_rites_castable_with_no_targets() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Empty graveyards — no valid targets
    let rites = castable_spell(&mut state, &registry, "Unburial Rites", P0);

    // Check if Unburial Rites can be cast
    let can_cast = can_cast(&state, &registry, rites);

    // BUG: Can cast with no legal targets because target_requirement is None
    assert!(!can_cast,
        "Unburial Rites should not be castable with no creature cards in any graveyard");
}

/// Bug: Into the Maw of Hell's `is_valid_target` accepts creatures for
/// the land target slot. Oracle says "Destroy target land" — the first
/// target must be a land, not a creature.
#[test]
fn bug_into_the_maw_accepts_creatures_as_land_target() {
    let registry = CardRegistry::with_all_cards();
    let state = game_at_step(Step::PrecombatMain, P0);

    let behavior = registry.get(
        registry.get_id_by_name("Into the Maw of Hell").unwrap()
    ).unwrap();

    // A creature should NOT be a valid target for the land slot
    let creature = Target::Object(ready_creature(&mut state.clone(), P1, 3, 3));
    let is_valid = behavior.is_valid_target(&state, P0, &creature, &registry);

    // BUG: Creatures are accepted as valid targets
    assert!(!is_valid,
        "Into the Maw of Hell should only target lands, not creatures");
}

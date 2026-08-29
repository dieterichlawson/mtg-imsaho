//! Integration tests for `mtg_engine::funding::build_options` — exercises
//! the source-gathering + grouping logic against real card registrations.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::funding::{self, FundingCategory};
use mtg_engine::types::*;

#[test]
fn empty_battlefield_gives_only_pool() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 2);

    let options = funding::build_options(&state, P0, &registry);
    assert_eq!(options.groups.len(), 0);
    assert_eq!(options.pool.get(&ManaType::Red).copied(), Some(2));
    assert_eq!(options.max_x, 2);
}

#[test]
fn basics_group_by_name() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &registry, "Mountain", P0);
    named_permanent(&mut state, &registry, "Mountain", P0);
    named_permanent(&mut state, &registry, "Mountain", P0);
    named_permanent(&mut state, &registry, "Swamp", P0);

    let options = funding::build_options(&state, P0, &registry);
    // 2 groups: Mountain x3, Swamp x1. Both category Lands, mana_per_tap 1.
    assert_eq!(options.groups.len(), 2);
    let mountain = options.groups.iter().find(|g| g.name == "Mountain").unwrap();
    let swamp = options.groups.iter().find(|g| g.name == "Swamp").unwrap();
    assert_eq!(mountain.category, FundingCategory::Lands);
    assert_eq!(mountain.mana_per_tap, 1);
    assert_eq!(mountain.source_ids.len(), 3);
    assert_eq!(swamp.source_ids.len(), 1);
    assert_eq!(options.max_x, 4);
}

#[test]
fn tapped_lands_are_excluded() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let tapped = named_permanent(&mut state, &registry, "Mountain", P0);
    state.get_object_mut(tapped).unwrap().tapped = true;
    named_permanent(&mut state, &registry, "Mountain", P0);

    let options = funding::build_options(&state, P0, &registry);
    let mountain = options.groups.iter().find(|g| g.name == "Mountain").unwrap();
    assert_eq!(mountain.source_ids.len(), 1, "tapped land should not appear");
    assert_eq!(options.max_x, 1);
}

#[test]
fn pool_plus_taps_determines_max_x() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);
    named_permanent(&mut state, &registry, "Swamp", P0);
    named_permanent(&mut state, &registry, "Swamp", P0);

    let options = funding::build_options(&state, P0, &registry);
    assert_eq!(options.max_x, 4); // 2 pool + 2 swamps
    assert_eq!(options.pool.get(&ManaType::Black).copied(), Some(2));
}

// ── The response arithmetic and validation bounds ───────────────────────
//
// The full mutation sweep (issues #26–#34) left the FundingResponse
// getters and validate()'s bounds unpinned.

/// x_value is pool drain plus tap output; tap_total counts only taps;
/// is_empty means exactly X = 0.
#[test]
fn a_funding_responses_arithmetic_adds_pool_and_taps() {
    use mtg_engine::funding::FundingResponse;
    use std::collections::BTreeMap;

    let mut pool = BTreeMap::new();
    pool.insert(ManaType::Green, 2);
    let mut taps = BTreeMap::new();
    taps.insert("Forest".to_string(), 1);
    let r = FundingResponse { pool, taps };
    assert_eq!(r.x_value(), 3, "2 from the pool + 1 from taps");
    assert_eq!(r.tap_total(), 1, "taps alone");
    assert!(!r.is_empty());

    let empty = FundingResponse::default();
    assert_eq!(empty.x_value(), 0);
    assert_eq!(empty.tap_total(), 0);
    assert!(empty.is_empty());
}

/// validate() refuses a tap allocation above the group's ceiling and a
/// pool drain above the floating mana — and accepts amounts exactly at
/// both bounds.
#[test]
fn funding_validation_enforces_its_bounds_exactly() {
    use mtg_engine::funding::{FundingCategory, FundingGroup, FundingOptions, FundingResponse, validate};
    use std::collections::BTreeMap;
    use mtg_engine::ids::ObjectId;

    let group = FundingGroup {
        name: "Forest".into(),
        category: FundingCategory::Lands,
        mana_per_tap: 1,
        source_ids: vec![ObjectId(1), ObjectId(2)],
        colors_produced: vec![Color::Green],
    };
    let mut pool = BTreeMap::new();
    pool.insert(ManaType::Green, 1);
    let options = FundingOptions { pool, groups: vec![group], max_x: 3 };

    let ok = |taps_amt: u32, pool_amt: u32| {
        let mut taps = BTreeMap::new();
        if taps_amt > 0 { taps.insert("Forest".to_string(), taps_amt); }
        let mut pool = BTreeMap::new();
        if pool_amt > 0 { pool.insert(ManaType::Green, pool_amt); }
        validate(&FundingResponse { pool, taps }, &options)
    };

    assert!(ok(2, 1).is_ok(), "both bounds exactly met is legal (X = max_x = 3)");
    assert!(ok(3, 0).is_err(), "three taps from a two-source group overflows");
    assert!(ok(0, 2).is_err(), "draining 2 from a pool of 1 overdraws");
}

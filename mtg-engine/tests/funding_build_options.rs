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
    let options = FundingOptions { pool, groups: vec![group], max_x: 3, x_discount: 0 };

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

/// `funding::apply` is the half that spends: it drains the pool the player
/// named, taps the sources the group allocations pay for, and drains what
/// those taps produced. Its contract is one sentence — after it runs, X is
/// what the response said and the pool has lost exactly the mana the response
/// named from it, no more — and everything inside is arithmetic serving that.
///
/// Checked over every valid response to one board rather than a handful of
/// chosen ones, because the interesting failures are off-by-one: a group whose
/// sources produce two mana at a time (Sol Ring) taps `amount / 2` of them,
/// and the mana the taps made is drained back out rather than left floating
/// for the next spell.
#[test]
fn applying_a_funding_response_spends_exactly_what_it_named() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &registry, "Forest", P0);
    named_permanent(&mut state, &registry, "Forest", P0);
    named_permanent(&mut state, &registry, "Sol Ring", P0);
    named_permanent(&mut state, &registry, "Sol Ring", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);

    let options = funding::build_options(&state, P0, &registry);
    let per_tap: Vec<u32> = options.groups.iter().map(|g| g.mana_per_tap).collect();
    assert!(per_tap.contains(&2), "the board carries a two-mana group: {per_tap:?}");
    assert!(per_tap.contains(&1), "and a one-mana group: {per_tap:?}");

    // Every allocation the options admit: each group at 0, per_tap, 2*per_tap,
    // ... up to its ceiling, crossed with every pool drain.
    let group_choices: Vec<Vec<(String, u32)>> = options.groups.iter()
        .map(|g| (0..=g.source_ids.len())
            .map(|n| (g.name.clone(), u32::try_from(n).unwrap() * g.mana_per_tap))
            .collect())
        .collect();
    let pool_choices: Vec<Vec<(ManaType, u32)>> = options.pool.iter()
        .map(|(&mt, &avail)| (0..=avail).map(|n| (mt, n)).collect())
        .collect();

    let mut checked = 0usize;
    for taps in cartesian(&group_choices) {
        for drains in cartesian(&pool_choices) {
            let response = funding::FundingResponse {
                pool: drains.iter().copied().filter(|&(_, n)| n > 0).collect(),
                taps: taps.iter().cloned().filter(|&(_, n)| n > 0).collect(),
            };
            if funding::validate(&response, &options).is_err() {
                continue;
            }
            let mut after = state.clone();
            let x = funding::apply(&mut after, P0, &options, &response, &registry);
            checked += 1;

            assert_eq!(x, response.x_value(), "X is the response's own sum: {response:?}");

            // Every type, not just the ones the pool started with: what the
            // taps produced has to be drained too, and a Forest produces green
            // into a pool that had none.
            for mt in [ManaType::White, ManaType::Blue, ManaType::Black,
                       ManaType::Red, ManaType::Green, ManaType::Colorless] {
                let before = options.pool.get(&mt).copied().unwrap_or(0);
                let drained = response.pool.get(&mt).copied().unwrap_or(0);
                assert_eq!(after.get_player(P0).mana_pool.get(mt), before - drained,
                    "{mt:?} lost exactly what {response:?} named — the taps paid for X, \
                     so what they produced is gone too, not left floating");
            }

            for group in &options.groups {
                let allocated = response.taps.get(&group.name).copied().unwrap_or(0);
                let expected = (allocated / group.mana_per_tap) as usize;
                let tapped = group.source_ids.iter()
                    .filter(|&&id| after.get_object(id).is_some_and(|o| o.tapped))
                    .count();
                assert_eq!(tapped, expected,
                    "{} funds {allocated} by tapping {expected} of its sources, \
                     each worth {}: {response:?}", group.name, group.mana_per_tap);
                assert!(group.source_ids.iter().take(expected)
                    .all(|&id| after.get_object(id).is_some_and(|o| o.tapped)),
                    "and it taps the first ones, so a plan is reproducible");
            }
        }
    }
    assert!(checked > 20, "the sweep covered {checked} responses, which is too few");
}

/// Every way to pick one entry from each list.
fn cartesian<T: Clone>(lists: &[Vec<T>]) -> Vec<Vec<T>> {
    let mut out = vec![vec![]];
    for list in lists {
        out = out.iter()
            .flat_map(|prefix| list.iter().map(move |item| {
                let mut next = prefix.clone();
                next.push(item.clone());
                next
            }))
            .collect();
    }
    out
}

/// A funding group says which colours its sources make, each once.
///
/// `colors_produced` is what a seat reads to know what a plan will actually
/// pay for — the LLM seat prints it beside the group — so a group that lists
/// nothing describes a source that makes no coloured mana, which is a
/// different offer from the one the board is making.
#[test]
fn a_funding_group_lists_every_colour_its_sources_make() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // Hinterland Harbor taps for {G} or for {U}: two abilities, one group,
    // two colours. A Forest is the one-colour half of the same question.
    named_permanent(&mut state, &registry, "Hinterland Harbor", P0);
    named_permanent(&mut state, &registry, "Forest", P0);

    let options = funding::build_options(&state, P0, &registry);
    let colours = |name: &str| {
        let mut c = options.groups.iter()
            .find(|g| g.name.contains(name))
            .unwrap_or_else(|| panic!("{name} is a funding source: {:?}",
                options.groups.iter().map(|g| &g.name).collect::<Vec<_>>()))
            .colors_produced.clone();
        c.sort_by_key(|c| format!("{c:?}"));
        c
    };

    assert_eq!(colours("Hinterland Harbor"), vec![Color::Blue, Color::Green],
        "both colours the land makes, each said once");
    assert_eq!(colours("Forest"), vec![Color::Green]);
}

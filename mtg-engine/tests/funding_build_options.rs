//! Integration tests for `mtg_engine::funding::build_options` — exercises
//! the source-gathering + grouping logic against real card registrations.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::funding::{self, FundingCategory};
use mtg_engine::types::*;

fn place_land(
    state: &mut mtg_engine::state::GameState,
    registry: &CardRegistry,
    name: &str,
    owner: mtg_engine::ids::PlayerId,
) -> mtg_engine::ids::ObjectId {
    let card_id = registry
        .get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.summoning_sick = false;
    id
}

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
    place_land(&mut state, &registry, "Mountain", P0);
    place_land(&mut state, &registry, "Mountain", P0);
    place_land(&mut state, &registry, "Mountain", P0);
    place_land(&mut state, &registry, "Swamp", P0);

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
    let tapped = place_land(&mut state, &registry, "Mountain", P0);
    state.get_object_mut(tapped).unwrap().tapped = true;
    place_land(&mut state, &registry, "Mountain", P0);

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
    place_land(&mut state, &registry, "Swamp", P0);
    place_land(&mut state, &registry, "Swamp", P0);

    let options = funding::build_options(&state, P0, &registry);
    assert_eq!(options.max_x, 4); // 2 pool + 2 swamps
    assert_eq!(options.pool.get(&ManaType::Black).copied(), Some(2));
}

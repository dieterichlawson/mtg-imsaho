//! Tapping lands for mana, one at a time.
//!
//! Reported from a real game: with four Swamps and a Stensia Bloodhall out,
//! tapping a Swamp and the Bloodhall appeared to produce nothing, and only
//! tapping two more Swamps made mana show up. The cause was that mana
//! abilities were being deduplicated by their *description* — every Swamp
//! offers the same "{T}: Add {B}" — so the four Swamps collapsed to one
//! offer and the rest were unreachable.

mod common;
use common::*;

use mtg_engine::types::*;

/// A board with several lands of one type plus one that produces something
/// else, which is the shape the report was about.
fn lands_out() -> (mtg_engine::state::GameState, mtg_engine::cards::CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..4 {
        named_permanent(&mut state, &reg, "Swamp", P0);
    }
    for _ in 0..2 {
        named_permanent(&mut state, &reg, "Mountain", P0);
    }
    named_permanent(&mut state, &reg, "Stensia Bloodhall", P0);
    (state, reg)
}

/// Each land adds its own mana, and the pool accumulates across taps of
/// different lands.
#[test]
fn tapping_lands_of_different_types_accumulates_their_mana() {
    let (state, reg) = lands_out();

    let state = tap_for_mana(&state, &reg, "Swamp");
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Black), 1, "the Swamp's {{B}}");

    let state = tap_for_mana(&state, &reg, "Stensia Bloodhall");
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Black), 1, "still there");
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Colorless), 1,
        "and the Bloodhall's {{C}} beside it");
}

/// Four Swamps are four mana abilities, not one. They print the same text, and
/// deduplicating the *offers* by that text made three of them unreachable.
#[test]
fn every_copy_of_a_land_can_be_tapped_in_turn() {
    let (mut state, reg) = lands_out();

    for n in 1..=4 {
        state = tap_for_mana(&state, &reg, "Swamp");
        assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Black), n,
            "after tapping {n} Swamp(s)");
    }

    assert!(!offers_mana_ability_for(&state, &reg, "Swamp"),
        "and once all four are tapped there is nothing left to offer");
}

/// Mana accumulated across several taps pays for a spell that no single land
/// could. The report's symptom was really this: the spell stayed uncastable
/// however much the player tapped.
#[test]
fn a_spell_becomes_castable_as_the_taps_accumulate() {
    let (mut state, reg) = lands_out();
    // Falkenrath Marauders is {3}{R}{R}: two red and three of anything.
    let marauders = spell_in_hand(&mut state, &reg, "Falkenrath Marauders", P0);

    assert!(can_cast(&state, &reg, marauders),
        "with seven untapped lands the engine can autotap for it from the start");

    for _ in 0..4 {
        state = tap_for_mana(&state, &reg, "Swamp");
    }
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Black), 4);
    assert!(can_cast(&state, &reg, marauders),
        "four {{B}} floating plus two untapped Mountains still covers {{3}}{{R}}{{R}}");
}

/// Whether the engine currently offers a mana ability for an untapped `name`.
fn offers_mana_ability_for(
    state: &mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    name: &str,
) -> bool {
    mtg_engine::engine::legal_actions(state, reg).actions.iter().any(|a| match a {
        mtg_engine::actions::Action::ActivateManaAbility { object_id, .. } => state
            .get_object(*object_id)
            .and_then(|o| reg.card_data(o.card_id))
            .is_some_and(|d| d.name == name),
        _ => false,
    })
}

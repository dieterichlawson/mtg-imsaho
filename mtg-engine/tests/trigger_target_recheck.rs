//! CR 608.2b: when a triggered ability tries to resolve, its targets are
//! re-checked. If they have all become illegal, the ability is countered by
//! the game rules.
//!
//! The re-check ran only the generic half — zone, hexproof, target filter —
//! and skipped `is_valid_target`, the card's own restriction on what it may
//! target. `resolve_spell` had always run both. So a trigger resolved happily
//! against a target that had stopped satisfying the card's wording: Grimgrin's
//! "creature the defending player controls" survived that creature changing
//! controller in response.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Angel of Flight Alabaster targets "a Spirit card in your graveyard". A
/// card that stops being a legal target between announcement and resolution
/// makes the ability fizzle.
#[test]
fn a_trigger_fizzles_when_its_target_stops_satisfying_the_cards_restriction() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    // A card in the graveyard that is NOT a Spirit. It satisfies the generic
    // half of legality — right zone, no hexproof, matches the target filter —
    // and is rejected only by the card's own `is_valid_target` ("target Spirit
    // card"). That is precisely the half the re-check used to skip.
    let not_a_spirit = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert!(!state.has_subtype(not_a_spirit, "Spirit", &reg), "test precondition");

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: angel,
        card_id: angel_card,
        controller: P0,
        description: "Angel of Flight Alabaster".into(),
        chosen_targets: vec![Target::Object(not_a_spirit)],
    }));
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(not_a_spirit).unwrap().zone, Zone::Graveyard,
        "the only target does not satisfy 'target Spirit card', so the ability \
         is countered on resolution rather than returning it (CR 608.2b)");
}

/// The happy path still works: a legal target is still returned.
#[test]
fn a_trigger_with_a_still_legal_target_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: angel,
        card_id: angel_card,
        controller: P0,
        description: "Angel of Flight Alabaster".into(),
        chosen_targets: vec![Target::Object(spirit)],
    }));
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "a legal Spirit card in the graveyard is returned to hand");
}

/// Civilized Scholar's "unless it attacked this turn" marker is stamped with
/// the turn it happened on. A bare marker could not be told apart from one
/// left over from a previous turn, and the clearing path only ran on the back
/// face's end step — so a front-face attack in turn N stuck forever and
/// stopped the Brute transforming back in every later turn.
#[test]
fn an_attack_in_an_earlier_turn_does_not_keep_the_brute_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let scholar = named_creature(&mut state, &reg, "Civilized Scholar", P0);
    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();

    // It attacked on the front face this turn...
    behavior.on_attacks(&mut state, scholar, AttackInfo::new(scholar, P1), &[], &reg);
    // ...then a later turn begins, and it transforms.
    state.turn_number += 1;
    mtg_engine::cards::helpers::apply_transform(&mut state, scholar, &reg);
    assert!(state.get_object(scholar).unwrap().is_transformed, "test precondition");

    behavior.on_end_step(&mut state, scholar, &[], &reg);

    assert!(!state.get_object(scholar).unwrap().is_transformed,
        "the attack was in a PREVIOUS turn, so Homicidal Brute did not attack \
         this turn and must tap and transform back");
}

/// The same-turn case still holds: an attack this turn keeps it transformed
/// (CR 711.5 — transforming does not make a new object).
#[test]
fn an_attack_this_turn_keeps_the_brute_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let scholar = named_creature(&mut state, &reg, "Civilized Scholar", P0);
    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();

    behavior.on_attacks(&mut state, scholar, AttackInfo::new(scholar, P1), &[], &reg);
    mtg_engine::cards::helpers::apply_transform(&mut state, scholar, &reg);
    behavior.on_end_step(&mut state, scholar, &[], &reg);

    assert!(state.get_object(scholar).unwrap().is_transformed,
        "it attacked this turn, so it stays a Homicidal Brute");
}

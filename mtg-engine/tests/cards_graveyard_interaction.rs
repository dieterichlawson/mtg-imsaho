//! Cards that read, return from, or exile a graveyard.
//!
//! Cards covered (9), so this is greppable by name as well as by rule:
//!
//! - Caravan Vigil
//! - Ghoulraiser
//! - Grasp of Phantoms
//! - Makeshift Mauler
//! - Mulch
//! - Night Terrors
//! - Purify the Grave
//! - Skaab Goliath
//! - Stitched Drake
//! - Woodland Sleuth
//!
//! Ghoulcaller's Chant and Memory's Journey have their own files; the copies
//! that lived here tested nothing those did not.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::triggers;
use mtg_engine::types::*;
// ═══════════════════════════════════════════════════════════════════
// "As an additional cost to cast this spell, exile N creature cards
// from your graveyard" (CR 601.2f)
// ═══════════════════════════════════════════════════════════════════

/// Three cards, one additional cost. The cost is paid as the spell is cast, so
/// by the time it resolves the fuel is already in exile — which is the part a
/// test that only checked the creature arrived would miss.
#[test]
fn exiling_creature_cards_pays_for_the_skaab() {
    // (spell, creature cards it exiles, its power/toughness)
    const CARDS: &[(&str, &[&str], i32, i32)] = &[
        ("Makeshift Mauler", &["Walking Corpse"], 4, 5),
        ("Stitched Drake", &["Grizzly Bears"], 3, 4),
        ("Skaab Goliath", &["Walking Corpse", "Grizzly Bears"], 6, 9),
    ];

    for &(spell_name, fuel_names, power, toughness) in CARDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let fuel: Vec<ObjectId> = fuel_names.iter()
            .map(|n| named_card_in_graveyard(&mut state, &reg, n, P0))
            .collect();

        let spell = castable_spell(&mut state, &reg, spell_name, P0);
        let state = cast_and_resolve(&state, &reg, spell, vec![]);

        assert_eq!(state.get_object(spell).unwrap().zone, Zone::Battlefield,
            "{spell_name} resolves onto the battlefield");
        for (&id, name) in fuel.iter().zip(fuel_names) {
            assert_eq!(state.get_object(id).unwrap().zone, Zone::Exile,
                "{spell_name} exiled {name} to pay its additional cost");
        }
        assert_eq!(state.effective_power(spell, &reg), Some(power), "{spell_name}'s power");
        assert_eq!(state.effective_toughness(spell, &reg), Some(toughness), "{spell_name}'s toughness");
    }
}

/// Stitched Drake's flying, asked of the game rather than of the card data —
/// `has_keyword` is zone-gated and consults the active face, so reading
/// `card_data().keywords` back would not exercise any of that.
#[test]
fn stitched_drake_has_flying_on_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let drake = castable_spell(&mut state, &reg, "Stitched Drake", P0);
    let state = cast_and_resolve(&state, &reg, drake, vec![]);

    assert!(state.has_keyword(drake, Keyword::Flying, &reg));
}

// ═══════════════════════════════════════════════════════════════════
// Ghoulraiser
// ═══════════════════════════════════════════════════════════════════

#[test]
fn ghoulraiser_returns_zombie_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a Zombie in P0's graveyard.
    let zombie = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    // Cast Ghoulraiser.
    let raiser = castable_spell(&mut state, &reg, "Ghoulraiser", P0);
    let mut new_state = cast_and_resolve(&state, &reg, raiser, vec![]);
    triggers::process_triggers(&mut new_state, &reg);

    // Ghoulraiser should be on the battlefield.
    assert_eq!(new_state.get_object(raiser).unwrap().zone, Zone::Battlefield);
    // Zombie should be returned to hand.
    assert_eq!(new_state.get_object(zombie).unwrap().zone, Zone::Hand);
}

// ═══════════════════════════════════════════════════════════════════
// Caravan Vigil
// ═══════════════════════════════════════════════════════════════════

/// "Search your library for a basic land card, reveal it, put it into your hand,
/// then shuffle. Morbid — Put that card onto the battlefield instead if a
/// creature died this turn."
///
/// The morbid half is a "you may", so with morbid on the game must ask; without
/// it, it must not.
#[test]
fn caravan_vigil_offers_the_battlefield_only_if_a_creature_died() {
    for died in [false, true] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = died;

        let forest = stock_library(&mut state, &reg, P0, 1)[0];
        let vigil = castable_spell(&mut state, &reg, "Caravan Vigil", P0);
        let mut state = cast_and_resolve(&state, &reg, vigil, vec![]);

        if died {
            assert!(state.awaiting_action.is_some(),
                "with a creature dead this turn, the morbid choice has to be offered");
            state = mtg_engine::engine::submit_action(
                &state, &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) }, &reg);
            assert_eq!(state.get_object(forest).unwrap().zone, Zone::Battlefield,
                "saying yes puts the land onto the battlefield");
        } else {
            assert!(state.awaiting_action.is_none(),
                "without morbid there is nothing to choose");
            assert_eq!(state.get_object(forest).unwrap().zone, Zone::Hand,
                "the land goes to hand");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Mulch
// ═══════════════════════════════════════════════════════════════════

#[test]
fn mulch_puts_lands_in_hand_and_rest_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up library with 4 cards: 2 lands, 2 non-lands.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let bear_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();

    let land1 = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(land1).unwrap().name = "Forest".into();
    let nonland1 = state.create_object(bear_card_id, P0, Zone::Library, Some(2), Some(2));
    state.get_object_mut(nonland1).unwrap().name = "Grizzly Bears".into();
    let land2 = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(land2).unwrap().name = "Forest".into();
    let nonland2 = state.create_object(bear_card_id, P0, Zone::Library, Some(2), Some(2));
    state.get_object_mut(nonland2).unwrap().name = "Grizzly Bears".into();

    state.get_player_mut(P0).library_order = vec![land1, nonland1, land2, nonland2];

    let mulch = castable_spell(&mut state, &reg, "Mulch", P0);
    let new_state = cast_and_resolve(&state, &reg, mulch, vec![]);

    // Lands should be in hand, non-lands in graveyard.
    assert_eq!(new_state.get_object(land1).unwrap().zone, Zone::Hand);
    assert_eq!(new_state.get_object(land2).unwrap().zone, Zone::Hand);
    assert_eq!(new_state.get_object(nonland1).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(nonland2).unwrap().zone, Zone::Graveyard);
}

// ═══════════════════════════════════════════════════════════════════
// Purify the Grave
// ═══════════════════════════════════════════════════════════════════

#[test]
fn purify_the_grave_exiles_card_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a card in P1's graveyard.
    let gy_card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);

    let purify = castable_spell(&mut state, &reg, "Purify the Grave", P0);
    let new_state = cast_and_resolve(&state, &reg, purify, vec![mtg_engine::actions::Target::Object(gy_card)]);

    assert_eq!(new_state.get_object(gy_card).unwrap().zone, Zone::Exile);
}

// ═══════════════════════════════════════════════════════════════════
// Grasp of Phantoms
// ═══════════════════════════════════════════════════════════════════

#[test]
fn grasp_of_phantoms_puts_creature_on_top_of_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target_creature = named_permanent(&mut state, &reg, "Grizzly Bears", P1);

    let grasp = castable_spell(&mut state, &reg, "Grasp of Phantoms", P0);
    let new_state = cast_and_resolve(&state, &reg, grasp, vec![Target::Object(target_creature)]);

    // Creature should be in library.
    assert_eq!(new_state.get_object(target_creature).unwrap().zone, Zone::Library);
    // It should be on top of the library.
    assert_eq!(new_state.get_player(P1).library_order[0], target_creature);
}

// ═══════════════════════════════════════════════════════════════════
// Night Terrors
// ═══════════════════════════════════════════════════════════════════

/// "Target player reveals their hand. You choose a nonland card from it. Exile
/// that card." One hand holding both kinds, so the test shows the choice being
/// made rather than just something being exiled — and shows that a hand with no
/// nonland loses nothing.
#[test]
fn night_terrors_takes_the_nonland_and_leaves_the_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let forest = spell_in_hand(&mut state, &reg, "Forest", P1);

    let terrors = castable_spell(&mut state, &reg, "Night Terrors", P0);
    let state = cast_and_resolve(&state, &reg, terrors, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Exile, "the nonland is taken");
    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Hand, "the land is not");
}

/// A hand of nothing but lands: the spell resolves and takes nothing.
#[test]
fn night_terrors_takes_nothing_from_a_hand_of_lands() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest = spell_in_hand(&mut state, &reg, "Forest", P1);

    let terrors = castable_spell(&mut state, &reg, "Night Terrors", P0);
    let state = cast_and_resolve(&state, &reg, terrors, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Hand);
}

// ═══════════════════════════════════════════════════════════════════
// Woodland Sleuth
// ═══════════════════════════════════════════════════════════════════

/// "Morbid — When Woodland Sleuth enters the battlefield, if a creature died
/// this turn, return a creature card from your graveyard to your hand."
///
/// An intervening-if clause (CR 603.4): both arms in one test, because "it
/// returned the creature" alone would pass for a card that returned it always.
#[test]
fn woodland_sleuth_returns_a_creature_only_if_one_died_this_turn() {
    for (died, expected) in [(true, Zone::Hand), (false, Zone::Graveyard)] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = died;

        let gy_creature = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
        let sleuth = castable_spell(&mut state, &reg, "Woodland Sleuth", P0);
        let mut state = cast_and_resolve(&state, &reg, sleuth, vec![]);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(state.get_object(sleuth).unwrap().zone, Zone::Battlefield,
            "the Sleuth arrives whether or not morbid is on");
        assert_eq!(state.get_object(gy_creature).unwrap().zone, expected,
            "creature_died_this_turn = {died}");
    }
}

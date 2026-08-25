//! Cards that read, return from, or exile a graveyard.
//!
//! Cards covered (12), so this is greppable by name as well as by rule:
//!
//! - Caravan Vigil
//! - Ghoulcaller's Chant
//! - Ghoulraiser
//! - Grasp of Phantoms
//! - Makeshift Mauler
//! - Memory's Journey
//! - Mulch
//! - Night Terrors
//! - Purify the Grave
//! - Skaab Goliath
//! - Stitched Drake
//! - Woodland Sleuth

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::triggers;
use mtg_engine::types::*;
// ═══════════════════════════════════════════════════════════════════
// Makeshift Mauler
// ═══════════════════════════════════════════════════════════════════

#[test]
fn makeshift_mauler_exiles_creature_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a creature card in P0's graveyard.
    let graveyard_creature = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(graveyard_creature, Zone::Graveyard, &reg);

    // Cast Makeshift Mauler.
    let mauler = castable_spell(&mut state, &reg, "Makeshift Mauler", P0);
    let new_state = cast_and_resolve(&state, &reg, mauler, vec![]);

    // Mauler should be on the battlefield.
    assert_eq!(new_state.get_object(mauler).unwrap().zone, Zone::Battlefield);
    // Graveyard creature should be exiled.
    assert_eq!(new_state.get_object(graveyard_creature).unwrap().zone, Zone::Exile);
}

#[test]
fn makeshift_mauler_is_4_5_zombie() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let graveyard_creature = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(graveyard_creature, Zone::Graveyard, &reg);

    let mauler = castable_spell(&mut state, &reg, "Makeshift Mauler", P0);
    let new_state = cast_and_resolve(&state, &reg, mauler, vec![]);

    assert_eq!(new_state.effective_power(mauler, &reg).unwrap(), 4);
    assert_eq!(new_state.effective_toughness(mauler, &reg).unwrap(), 5);
}

// ═══════════════════════════════════════════════════════════════════
// Stitched Drake
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stitched_drake_exiles_creature_and_has_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let graveyard_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(graveyard_creature, Zone::Graveyard, &reg);

    let drake = castable_spell(&mut state, &reg, "Stitched Drake", P0);
    let new_state = cast_and_resolve(&state, &reg, drake, vec![]);

    assert_eq!(new_state.get_object(drake).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(graveyard_creature).unwrap().zone, Zone::Exile);
    assert_eq!(new_state.effective_power(drake, &reg).unwrap(), 3);
    assert_eq!(new_state.effective_toughness(drake, &reg).unwrap(), 4);
    // Flying is checked via keywords on the card data.
    let card_data = reg.card_data(new_state.get_object(drake).unwrap().card_id).unwrap();
    assert!(card_data.keywords.contains(&Keyword::Flying));
}

// ═══════════════════════════════════════════════════════════════════
// Skaab Goliath
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skaab_goliath_exiles_two_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gy1 = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(gy1, Zone::Graveyard, &reg);
    let gy2 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(gy2, Zone::Graveyard, &reg);

    let goliath = castable_spell(&mut state, &reg, "Skaab Goliath", P0);
    let new_state = cast_and_resolve(&state, &reg, goliath, vec![]);

    assert_eq!(new_state.get_object(goliath).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(gy1).unwrap().zone, Zone::Exile);
    assert_eq!(new_state.get_object(gy2).unwrap().zone, Zone::Exile);
    assert_eq!(new_state.effective_power(goliath, &reg).unwrap(), 6);
    assert_eq!(new_state.effective_toughness(goliath, &reg).unwrap(), 9);
}

// ═══════════════════════════════════════════════════════════════════
// Ghoulcaller's Chant
// ═══════════════════════════════════════════════════════════════════

#[test]
fn ghoulcallers_chant_returns_creature_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gy_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(gy_creature, Zone::Graveyard, &reg);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let new_state = cast_and_resolve(&state, &reg, chant, vec![mtg_engine::actions::Target::Object(gy_creature)]);

    // Creature should be back in hand.
    assert_eq!(new_state.get_object(gy_creature).unwrap().zone, Zone::Hand);
}

#[test]
fn ghoulcallers_chant_returns_two_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put two Zombies in graveyard.
    let zombie1 = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(zombie1, Zone::Graveyard, &reg);
    let zombie2 = spell_in_hand(&mut state, &reg, "Diregraf Ghoul", P0);
    state.move_object(zombie2, Zone::Graveyard, &reg);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let new_state = cast_and_resolve(&state, &reg, chant, vec![
        mtg_engine::actions::Target::Object(zombie1),
        mtg_engine::actions::Target::Object(zombie2),
    ]);

    // Both Zombies should be back in hand.
    assert_eq!(new_state.get_object(zombie1).unwrap().zone, Zone::Hand);
    assert_eq!(new_state.get_object(zombie2).unwrap().zone, Zone::Hand);
}

// ═══════════════════════════════════════════════════════════════════
// Ghoulraiser
// ═══════════════════════════════════════════════════════════════════

#[test]
fn ghoulraiser_returns_zombie_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a Zombie in P0's graveyard.
    let zombie = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(zombie, Zone::Graveyard, &reg);

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

#[test]
fn caravan_vigil_finds_basic_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a Forest in P0's library.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(forest);

    let vigil = castable_spell(&mut state, &reg, "Caravan Vigil", P0);
    let new_state = cast_and_resolve(&state, &reg, vigil, vec![]);

    // Forest should be in hand (no morbid).
    assert_eq!(new_state.get_object(forest).unwrap().zone, Zone::Hand);
}

#[test]
fn caravan_vigil_morbid_choose_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    // Put a Forest in P0's library.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(forest);

    let vigil = castable_spell(&mut state, &reg, "Caravan Vigil", P0);
    let mut new_state = cast_and_resolve(&state, &reg, vigil, vec![]);

    // Should be awaiting a YesNo choice for morbid "you may" put on battlefield.
    if new_state.awaiting_action.is_some() {
        // Accept the morbid choice (put on battlefield).
        new_state = mtg_engine::engine::submit_action(
            &new_state, &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) }, &reg,
        );
    }

    // Forest should be on the battlefield (morbid, player chose yes).
    assert_eq!(new_state.get_object(forest).unwrap().zone, Zone::Battlefield);
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
    let gy_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(gy_card, Zone::Graveyard, &reg);

    let purify = castable_spell(&mut state, &reg, "Purify the Grave", P0);
    let new_state = cast_and_resolve(&state, &reg, purify, vec![mtg_engine::actions::Target::Object(gy_card)]);

    assert_eq!(new_state.get_object(gy_card).unwrap().zone, Zone::Exile);
}

#[test]
fn purify_the_grave_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Purify the Grave").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some());
}

// ═══════════════════════════════════════════════════════════════════
// Grasp of Phantoms
// ═══════════════════════════════════════════════════════════════════

#[test]
fn grasp_of_phantoms_puts_creature_on_top_of_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target_creature = named_creature(&mut state, &reg, "Grizzly Bears", P1);

    let grasp = castable_spell(&mut state, &reg, "Grasp of Phantoms", P0);
    let new_state = cast_and_resolve(&state, &reg, grasp, vec![Target::Object(target_creature)]);

    // Creature should be in library.
    assert_eq!(new_state.get_object(target_creature).unwrap().zone, Zone::Library);
    // It should be on top of the library.
    assert_eq!(new_state.get_player(P1).library_order[0], target_creature);
}

#[test]
fn grasp_of_phantoms_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Grasp of Phantoms").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some());
}

// ═══════════════════════════════════════════════════════════════════
// Night Terrors
// ═══════════════════════════════════════════════════════════════════

#[test]
fn night_terrors_exiles_nonland_from_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a nonland card in P1's hand.
    let bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let terrors = castable_spell(&mut state, &reg, "Night Terrors", P0);
    let new_state = cast_and_resolve(&state, &reg, terrors, vec![Target::Player(P1)]);

    // The nonland card should be exiled.
    assert_eq!(new_state.get_object(bear).unwrap().zone, Zone::Exile);
}

#[test]
fn night_terrors_skips_lands() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put only a land in P1's hand.
    let forest = spell_in_hand(&mut state, &reg, "Forest", P1);

    let terrors = castable_spell(&mut state, &reg, "Night Terrors", P0);
    let new_state = cast_and_resolve(&state, &reg, terrors, vec![Target::Player(P1)]);

    // Land should still be in hand (no nonland to exile).
    assert_eq!(new_state.get_object(forest).unwrap().zone, Zone::Hand);
}

// ═══════════════════════════════════════════════════════════════════
// Memory's Journey
// ═══════════════════════════════════════════════════════════════════

#[test]
fn memorys_journey_shuffles_cards_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put cards in P1's graveyard.
    let gy1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(gy1, Zone::Graveyard, &reg);
    let gy2 = spell_in_hand(&mut state, &reg, "Walking Corpse", P1);
    state.move_object(gy2, Zone::Graveyard, &reg);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    // Must target a player (to determine whose graveyard) plus the cards.
    let new_state = cast_and_resolve(&state, &reg, journey, vec![
        Target::Player(P1),
        Target::Object(gy1),
        Target::Object(gy2),
    ]);

    // Both cards should be in library now.
    assert_eq!(new_state.get_object(gy1).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(gy2).unwrap().zone, Zone::Library);
}

#[test]
fn memorys_journey_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Memory's Journey").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some());
    // Flashback cost is {G}.
    assert_eq!(data.flashback_cost.unwrap().symbols.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════
// Woodland Sleuth
// ═══════════════════════════════════════════════════════════════════

#[test]
fn woodland_sleuth_morbid_returns_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    // Put a creature in P0's graveyard.
    let gy_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(gy_creature, Zone::Graveyard, &reg);

    let sleuth = castable_spell(&mut state, &reg, "Woodland Sleuth", P0);
    let mut new_state = cast_and_resolve(&state, &reg, sleuth, vec![]);
    triggers::process_triggers(&mut new_state, &reg);

    assert_eq!(new_state.get_object(sleuth).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(gy_creature).unwrap().zone, Zone::Hand);
}

#[test]
fn woodland_sleuth_no_morbid_no_return() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // creature_died_this_turn is false by default.

    let gy_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(gy_creature, Zone::Graveyard, &reg);

    let sleuth = castable_spell(&mut state, &reg, "Woodland Sleuth", P0);
    let mut new_state = cast_and_resolve(&state, &reg, sleuth, vec![]);
    triggers::process_triggers(&mut new_state, &reg);

    assert_eq!(new_state.get_object(sleuth).unwrap().zone, Zone::Battlefield);
    // Creature should still be in graveyard (no morbid).
    assert_eq!(new_state.get_object(gy_creature).unwrap().zone, Zone::Graveyard);
}

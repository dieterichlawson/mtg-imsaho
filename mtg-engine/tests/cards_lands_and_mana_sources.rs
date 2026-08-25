//! Tests for simple Innistrad cards: dual lands, utility lands, mana dorks,
//! artifacts, sorceries/instants, and enchantments.
//!
//! Cards covered (16), so this is greppable by name as well as by rule:
//!
//! - Avacyn's Pilgrim
//! - Deranged Assistant
//! - Full Moon's Rise
//! - Ghost Quarter
//! - Ghoulcaller's Bell
//! - Graveyard Shovel
//! - Into the Maw of Hell
//! - Make a Wish
//! - Maw of the Mire
//! - Moonmist
//! - Moorland Haunt
//! - Paraselene
//! - Runic Repetition
//! - Shimmering Grotto
//! - Stony Silence
//! - Witchbane Orb

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Dual Lands (checklands)
//
// All Innistrad checklands share one rule: "enters tapped unless you
// control an <A> or a <B>". Before adding a new dual land, extend
// DUAL_LANDS and the three parameterised tests below will cover it.
// ══════════════════════════════════════════════════════════════════

struct DualLand {
    name: &'static str,
    companion_a: &'static str,
    companion_b: &'static str,
}

const DUAL_LANDS: &[DualLand] = &[
    DualLand { name: "Clifftop Retreat",  companion_a: "Mountain", companion_b: "Plains" },
    DualLand { name: "Hinterland Harbor", companion_a: "Forest",   companion_b: "Island" },
    DualLand { name: "Isolated Chapel",   companion_a: "Plains",   companion_b: "Swamp" },
    DualLand { name: "Sulfur Falls",      companion_a: "Island",   companion_b: "Mountain" },
    DualLand { name: "Woodland Cemetery", companion_a: "Swamp",    companion_b: "Forest" },
];

/// Play the dual land, optionally with a companion basic already on the
/// battlefield, and return whether the dual entered tapped.
fn play_dual_and_check_tapped(
    reg: &CardRegistry,
    dual_name: &str,
    companion_basic: Option<&str>,
) -> bool {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    if let Some(basic_name) = companion_basic {
        let basic_id = reg.get_id_by_name(basic_name)
            .unwrap_or_else(|| panic!("{basic_name} must be registered"));
        let basic_obj = state.create_object(basic_id, P0, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(basic_obj).unwrap();
        obj.name = basic_name.into();
        obj.subtypes = vec![basic_name.into()];
    }

    let dual = spell_in_hand(&mut state, reg, dual_name, P0);
    state.get_player_mut(P0).land_plays_remaining = 1;

    let legal = engine::legal_actions(&state, reg);
    let play = legal.actions.iter()
        .find(|a| matches!(a, Action::PlayLand { object_id } if *object_id == dual))
        .unwrap_or_else(|| panic!("{dual_name} should be playable"));
    state = engine::submit_action(&state, play, reg);

    mtg_engine::triggers::collect_triggers(&mut state, reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, reg);

    state.get_object(dual).unwrap().tapped
}

#[test]
fn dual_lands_are_registered_as_lands() {
    let reg = registry();
    for case in DUAL_LANDS {
        let id = reg.get_id_by_name(case.name)
            .unwrap_or_else(|| panic!("{} should be registered", case.name));
        let data = reg.card_data(id)
            .unwrap_or_else(|| panic!("{} should have card data", case.name));
        assert!(data.card_types.contains(&CardType::Land),
            "{} should have CardType::Land", case.name);
        assert!(data.cost.is_none(),
            "{} should have no mana cost", case.name);
    }
}

#[test]
fn dual_lands_enter_tapped_without_matching_land() {
    let reg = registry();
    for case in DUAL_LANDS {
        let tapped = play_dual_and_check_tapped(&reg, case.name, None);
        assert!(tapped, "{} should enter tapped with no companion basic", case.name);
    }
}

#[test]
fn dual_lands_enter_untapped_with_matching_land() {
    let reg = registry();
    for case in DUAL_LANDS {
        for companion in [case.companion_a, case.companion_b] {
            let tapped = play_dual_and_check_tapped(&reg, case.name, Some(companion));
            assert!(!tapped,
                "{} should enter untapped with a {companion} on battlefield",
                case.name);
        }
    }
}

#[test]
fn dual_lands_have_two_mana_abilities() {
    let reg = registry();
    for case in DUAL_LANDS {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let card_id = reg.get_id_by_name(case.name).unwrap();
        let dual = state.create_object(card_id, P0, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(dual).unwrap();
        obj.name = case.name.into();
        obj.summoning_sick = false;

        let legal = engine::legal_actions(&state, &reg);
        let mana_actions: Vec<_> = legal.actions.iter()
            .filter(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == dual))
            .collect();
        assert_eq!(mana_actions.len(), 2,
            "{} should expose two mana abilities, got {}", case.name, mana_actions.len());
    }
}

// ══════════════════════════════════════════════════════════════════
// Ghost Quarter
// ══════════════════════════════════════════════════════════════════

#[test]
fn ghost_quarter_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Ghost Quarter").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Land));
    assert!(data.oracle_text.contains("Destroy target land"));
}

#[test]
fn ghost_quarter_taps_for_colorless() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gq_card_id = reg.get_id_by_name("Ghost Quarter").unwrap();
    let gq = state.create_object(gq_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(gq).unwrap().name = "Ghost Quarter".into();

    let legal = engine::legal_actions(&state, &reg);
    let mana_action = legal.actions.iter().find(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == gq));
    assert!(mana_action.is_some(), "Ghost Quarter should tap for colorless");
}

// ══════════════════════════════════════════════════════════════════
// Shimmering Grotto
// ══════════════════════════════════════════════════════════════════

#[test]
fn shimmering_grotto_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Shimmering Grotto").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Land));
}

#[test]
fn shimmering_grotto_taps_for_colorless() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sg_card_id = reg.get_id_by_name("Shimmering Grotto").unwrap();
    let sg = state.create_object(sg_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(sg).unwrap().name = "Shimmering Grotto".into();

    let legal = engine::legal_actions(&state, &reg);
    let mana_action = legal.actions.iter().find(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == sg));
    assert!(mana_action.is_some(), "Shimmering Grotto should tap for colorless");
}

// ══════════════════════════════════════════════════════════════════
// Moorland Haunt
// ══════════════════════════════════════════════════════════════════

#[test]
fn moorland_haunt_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Moorland Haunt").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Land));
    assert!(data.oracle_text.contains("Spirit"));
}

#[test]
fn moorland_haunt_creates_spirit_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mh_card_id = reg.get_id_by_name("Moorland Haunt").unwrap();
    let mh = state.create_object(mh_card_id, P0, Zone::Battlefield, None, None);
    let mh_obj = state.get_object_mut(mh).unwrap();
    mh_obj.name = "Moorland Haunt".into();
    mh_obj.summoning_sick = false; // Lands don't have summoning sickness

    // Put a creature in graveyard.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.move_object(creature, Zone::Graveyard, &reg);

    // Give mana.
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == mh));
    assert!(activate.is_some(), "Should be able to activate Moorland Haunt");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // Check a Spirit Token was created (Moorland Haunt creates a "Spirit Token" token).
    assert_eq!(count_tokens_named(&state, "Spirit Token"), 1, "Should create one Spirit token");
    let spirit = find_token_named(&state, "Spirit Token").unwrap();
    let obj = state.get_object(spirit).unwrap();
    assert_eq!(obj.power, Some(1));
    assert_eq!(obj.toughness, Some(1));
    assert!(obj.keywords.contains(&Keyword::Flying));

    // The creature should be exiled.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile);
}

// ══════════════════════════════════════════════════════════════════
// Avacyn's Pilgrim
// ══════════════════════════════════════════════════════════════════

#[test]
fn avacyns_pilgrim_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Avacyn's Pilgrim").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(1));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 1);
    assert!(data.subtypes.contains(&"Human".to_string()));
    assert!(data.subtypes.contains(&"Monk".to_string()));
}

#[test]
fn avacyns_pilgrim_taps_for_white() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);

    let legal = engine::legal_actions(&state, &reg);
    let mana_action = legal.actions.iter().find(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == pilgrim));
    assert!(mana_action.is_some(), "Avacyn's Pilgrim should tap for white");

    state = engine::submit_action(&state, mana_action.unwrap(), &reg);
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::White), 1);
}

#[test]
fn avacyns_pilgrim_cant_tap_with_summoning_sickness() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Avacyn's Pilgrim").unwrap();
    let pilgrim = state.create_object(card_id, P0, Zone::Battlefield, Some(1), Some(1));
    state.get_object_mut(pilgrim).unwrap().name = "Avacyn's Pilgrim".into();
    // summoning_sick = true by default

    let legal = engine::legal_actions(&state, &reg);
    let mana_action = legal.actions.iter().find(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == pilgrim));
    assert!(mana_action.is_none(), "Should not be able to tap with summoning sickness");
}

// ══════════════════════════════════════════════════════════════════
// Deranged Assistant
// ══════════════════════════════════════════════════════════════════

#[test]
fn deranged_assistant_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Deranged Assistant").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(1));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
    assert!(data.subtypes.contains(&"Human".to_string()));
    assert!(data.subtypes.contains(&"Wizard".to_string()));
}

#[test]
fn deranged_assistant_taps_for_colorless() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let assistant = named_creature(&mut state, &reg, "Deranged Assistant", P0);

    // Need at least one card in library for the mill cost.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let lib_card = state.create_object(forest_id, P0, Zone::Library, None, None);
    state.get_object_mut(lib_card).unwrap().name = "Forest".into();
    state.players[0].library_order = vec![lib_card];

    let legal = engine::legal_actions(&state, &reg);
    let mana_action = legal.actions.iter().find(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == assistant));
    assert!(mana_action.is_some(), "Deranged Assistant should tap for colorless");

    state = engine::submit_action(&state, mana_action.unwrap(), &reg);
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Colorless), 1);
}

// ══════════════════════════════════════════════════════════════════
// Ghoulcaller's Bell
// ══════════════════════════════════════════════════════════════════

#[test]
fn ghoulcallers_bell_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Ghoulcaller's Bell").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 1);
}

#[test]
fn ghoulcallers_bell_mills_both_players() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bell = named_creature(&mut state, &reg, "Ghoulcaller's Bell", P0);

    // Put cards in both libraries.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let p0_card = state.create_object(forest_id, P0, Zone::Library, None, None);
    state.get_object_mut(p0_card).unwrap().name = "Forest".into();
    state.players[0].library_order = vec![p0_card];

    let island_id = reg.get_id_by_name("Island").unwrap();
    let p1_card = state.create_object(island_id, P1, Zone::Library, None, None);
    state.get_object_mut(p1_card).unwrap().name = "Island".into();
    state.players[1].library_order = vec![p1_card];

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == bell));
    assert!(activate.is_some(), "Should be able to activate Ghoulcaller's Bell");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    // Both players should have had a card milled.
    assert_eq!(state.get_object(p0_card).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

// ══════════════════════════════════════════════════════════════════
// Graveyard Shovel
// ══════════════════════════════════════════════════════════════════

#[test]
fn graveyard_shovel_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Graveyard Shovel").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn graveyard_shovel_exiles_and_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_creature(&mut state, &reg, "Graveyard Shovel", P0);

    // Put a creature in graveyard.
    let creature = ready_creature(&mut state, P1, 3, 3);
    state.move_object(creature, Zone::Graveyard, &reg);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == shovel));
    assert!(activate.is_some(), "Should be able to activate Graveyard Shovel");

    state = engine::submit_action(&state, activate.unwrap(), &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_player(P0).life, 22, "Should gain 2 life for exiling a creature");
}

// ══════════════════════════════════════════════════════════════════
// Paraselene
// ══════════════════════════════════════════════════════════════════

#[test]
fn paraselene_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Paraselene").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Sorcery));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 3);
}

#[test]
fn paraselene_destroys_enchantments_and_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put two enchantments on the battlefield.
    let enc1_id = reg.get_id_by_name("Intangible Virtue").unwrap();
    let enc1 = state.create_object(enc1_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(enc1).unwrap().name = "Intangible Virtue".into();

    let enc2_id = reg.get_id_by_name("Glorious Anthem").unwrap();
    let enc2 = state.create_object(enc2_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(enc2).unwrap().name = "Glorious Anthem".into();

    let spell = castable_spell(&mut state, &reg, "Paraselene", P0);
    state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert_eq!(state.get_object(enc1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(enc2).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_player(P0).life, 22, "Should gain 1 life per enchantment destroyed");
}

// ══════════════════════════════════════════════════════════════════
// Into the Maw of Hell
// ══════════════════════════════════════════════════════════════════

#[test]
fn into_the_maw_of_hell_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Into the Maw of Hell").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Sorcery));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 6);
}

// ══════════════════════════════════════════════════════════════════
// Maw of the Mire
// ══════════════════════════════════════════════════════════════════

#[test]
fn maw_of_the_mire_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Maw of the Mire").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Sorcery));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 5);
}

#[test]
fn maw_of_the_mire_destroys_land_and_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Opponent has a land.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();

    let spell = castable_spell(&mut state, &reg, "Maw of the Mire", P0);
    state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(forest)]);

    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Graveyard, "Land should be destroyed");
    assert_eq!(state.get_player(P0).life, 24, "Should gain 4 life");
}

// ══════════════════════════════════════════════════════════════════
// Make a Wish
// ══════════════════════════════════════════════════════════════════

#[test]
fn make_a_wish_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Make a Wish").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Sorcery));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 4);
}

#[test]
fn make_a_wish_returns_cards_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 cards in graveyard.
    let c1 = ready_creature(&mut state, P0, 1, 1);
    state.move_object(c1, Zone::Graveyard, &reg);
    let c2 = ready_creature(&mut state, P0, 2, 2);
    state.move_object(c2, Zone::Graveyard, &reg);
    let c3 = ready_creature(&mut state, P0, 3, 3);
    state.move_object(c3, Zone::Graveyard, &reg);

    let spell = castable_spell(&mut state, &reg, "Make a Wish", P0);
    state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Should return exactly 2 cards to hand.
    let cards = [c1, c2, c3];
    let in_hand_count = cards.iter()
        .filter(|&&id| state.get_object(id).unwrap().zone == Zone::Hand)
        .count();
    assert_eq!(in_hand_count, 2, "Should return exactly 2 cards to hand");
}

// ══════════════════════════════════════════════════════════════════
// Moonmist
// ══════════════════════════════════════════════════════════════════

#[test]
fn moonmist_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Moonmist").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Instant));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

// ══════════════════════════════════════════════════════════════════
// Runic Repetition
// ══════════════════════════════════════════════════════════════════

#[test]
fn runic_repetition_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Runic Repetition").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Sorcery));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 3);
}

#[test]
fn runic_repetition_returns_flashback_card_from_exile() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Think Twice (has flashback) in exile.
    let tt_card_id = reg.get_id_by_name("Think Twice").unwrap();
    let tt = state.create_object(tt_card_id, P0, Zone::Exile, None, None);
    state.get_object_mut(tt).unwrap().name = "Think Twice".into();

    let spell = castable_spell(&mut state, &reg, "Runic Repetition", P0);
    state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(tt)]);

    assert_eq!(state.get_object(tt).unwrap().zone, Zone::Hand, "Should return flashback card to hand");
}

// ══════════════════════════════════════════════════════════════════
// Full Moon's Rise
// ══════════════════════════════════════════════════════════════════

#[test]
fn full_moons_rise_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Full Moon's Rise").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Enchantment));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
    assert!(!data.continuous_effects.is_empty());
}

// ══════════════════════════════════════════════════════════════════
// Stony Silence
// ══════════════════════════════════════════════════════════════════

#[test]
fn stony_silence_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Stony Silence").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Enchantment));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn stony_silence_blocks_artifact_mana_abilities() {
    // Per ruling: "No abilities of artifacts can be activated, including mana abilities."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put Sol Ring on the battlefield (artifact with mana ability).
    let sol_ring = named_creature(&mut state, &reg, "Sol Ring", P0);
    // Sol Ring isn't a creature — fix the card types.
    if let Some(obj) = state.get_object_mut(sol_ring) {
        obj.card_types = vec![CardType::Artifact];
        obj.power = None;
        obj.toughness = None;
    }

    // Without Stony Silence: Sol Ring's mana ability should be available.
    let actions_before = engine::legal_actions(&state, &reg);
    let has_mana_ability = actions_before.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == sol_ring));
    assert!(has_mana_ability, "Sol Ring mana ability should be available without Stony Silence");

    // Put Stony Silence on the battlefield.
    let _stony = named_creature(&mut state, &reg, "Stony Silence", P0);

    // With Stony Silence: Sol Ring's mana ability should be blocked.
    let actions_after = engine::legal_actions(&state, &reg);
    let has_mana_ability = actions_after.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == sol_ring));
    assert!(!has_mana_ability, "Sol Ring mana ability should be blocked by Stony Silence");
}

#[test]
fn stony_silence_does_not_block_non_artifact_mana() {
    // Stony Silence should NOT affect non-artifact mana abilities (lands, creatures).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put a Forest on the battlefield.
    let forest = named_creature(&mut state, &reg, "Forest", P0);
    if let Some(obj) = state.get_object_mut(forest) {
        obj.card_types = vec![CardType::Land];
        obj.power = None;
        obj.toughness = None;
    }

    // Put Stony Silence on the battlefield.
    let _stony = named_creature(&mut state, &reg, "Stony Silence", P0);

    // Forest mana ability should still work.
    let actions = engine::legal_actions(&state, &reg);
    let has_forest_mana = actions.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == forest));
    assert!(has_forest_mana, "Forest mana ability should NOT be blocked by Stony Silence");
}

// ══════════════════════════════════════════════════════════════════
// Witchbane Orb
// ══════════════════════════════════════════════════════════════════

#[test]
fn witchbane_orb_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Witchbane Orb").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 4);
}

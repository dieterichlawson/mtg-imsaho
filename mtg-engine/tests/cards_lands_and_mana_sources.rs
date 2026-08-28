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

    state = resolve_activated(engine::submit_action(&state, activate.unwrap(), &reg), &reg);

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
fn avacyns_pilgrim_taps_for_white() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);

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
fn deranged_assistant_taps_for_colorless() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let assistant = named_permanent(&mut state, &reg, "Deranged Assistant", P0);

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

    // "{T}, **Mill a card**: Add {C}." The mill is half the cost, and the
    // mana is worth nothing as evidence that it was paid.
    assert_eq!(state.get_object(lib_card).unwrap().zone, Zone::Graveyard,
        "the milled card is in the graveyard");
    assert!(state.get_player(P0).library_order.is_empty(), "and out of the library");
    assert!(state.get_object(assistant).unwrap().tapped, "and the Assistant is tapped");

    // With nothing left to mill the cost can no longer be paid, so the
    // ability is no longer offered (CR 701.17b).
    let legal = engine::legal_actions(&state, &reg);
    assert!(!legal.actions.iter().any(|a| matches!(a,
        Action::ActivateManaAbility { object_id, .. } if *object_id == assistant)));
}

/// CR 701.17b: "the player can't pay a cost that includes milling a number of
/// cards greater than the number of cards in their library."
///
/// A tap plan is worked out in full before any of it is executed. Two Deranged
/// Assistants over a one-card library are both offered — and after the first
/// mills that card, the second one's cost has become unpayable.
#[test]
fn a_second_deranged_assistant_cannot_mill_an_empty_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let first = named_permanent(&mut state, &reg, "Deranged Assistant", P0);
    let second = named_permanent(&mut state, &reg, "Deranged Assistant", P0);
    let lib_card = state.create_object(
        reg.get_id_by_name("Forest").unwrap(), P0, Zone::Library, None, None);
    state.players[0].library_order = vec![lib_card];

    // Both are available while the card is still there.
    for id in [first, second] {
        assert_eq!(mtg_engine::engine::available_mana_abilities(&state, id, &reg).len(), 1);
    }

    mtg_engine::engine::activate_mana_source(&mut state, first, 0, &reg);
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Colorless), 1);
    assert_eq!(state.get_object(lib_card).unwrap().zone, Zone::Graveyard);

    mtg_engine::engine::activate_mana_source(&mut state, second, 0, &reg);
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Colorless), 1,
        "the second Assistant's cost is unpayable, so it produces no mana");
    assert!(!state.get_object(second).unwrap().tapped, "and it is not tapped");
}

// ══════════════════════════════════════════════════════════════════
// Ghoulcaller's Bell
// ══════════════════════════════════════════════════════════════════

/// "{T}: Each player mills **a** card." Each is one card, from every player's
/// library — so each library needs a second card underneath, or milling two
/// looks exactly like milling one.
#[test]
fn ghoulcallers_bell_mills_both_players() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bell = named_permanent(&mut state, &reg, "Ghoulcaller's Bell", P0);

    // Two cards in each library: the top one is milled, the one under it is
    // the claim about how many.
    let library_of = |state: &mut mtg_engine::state::GameState, owner, name: &str| {
        let card_id = reg.get_id_by_name(name).unwrap();
        let ids: Vec<ObjectId> = (0..2).map(|_| {
            let id = state.create_object(card_id, owner, Zone::Library, None, None);
            state.get_object_mut(id).unwrap().name = name.into();
            id
        }).collect();
        state.get_player_mut(owner).library_order.clone_from(&ids);
        ids
    };
    let p0_cards = library_of(&mut state, P0, "Forest");
    let p1_cards = library_of(&mut state, P1, "Island");

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == bell));
    assert!(activate.is_some(), "Should be able to activate Ghoulcaller's Bell");

    state = resolve_activated(engine::submit_action(&state, activate.unwrap(), &reg), &reg);

    // Both players should have had exactly one card milled.
    assert_eq!(state.get_object(p0_cards[0]).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_cards[0]).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p0_cards[1]).unwrap().zone, Zone::Library,
        "\"mills **a** card\" — the one underneath stays put");
    assert_eq!(state.get_object(p1_cards[1]).unwrap().zone, Zone::Library);
}

/// CR 701.17b: milling as an *effect* mills as many as possible, so a player
/// with an empty library simply mills nothing — unlike a *cost* that includes
/// milling, which cannot be paid at all (Deranged Assistant).
#[test]
fn ghoulcallers_bell_mills_what_it_can_from_an_empty_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bell = named_permanent(&mut state, &reg, "Ghoulcaller's Bell", P0);
    // P0's library is empty; P1 has one card.
    let card_id = reg.get_id_by_name("Island").unwrap();
    let theirs = state.create_object(card_id, P1, Zone::Library, None, None);
    state.get_player_mut(P1).library_order = vec![theirs];

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == bell))
        .expect("still activatable — the mill is an effect, not a cost");
    let state = resolve_activated(engine::submit_action(&state, activate, &reg), &reg);

    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "the player who has a card still mills it");
    assert!(state.get_player(P0).library_order.is_empty());
}

// ══════════════════════════════════════════════════════════════════
// Graveyard Shovel
// ══════════════════════════════════════════════════════════════════

#[test]
fn graveyard_shovel_exiles_and_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shovel = named_permanent(&mut state, &reg, "Graveyard Shovel", P0);

    // Put a creature in graveyard.
    let creature = ready_creature(&mut state, P1, 3, 3);
    state.move_object(creature, Zone::Graveyard, &reg);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let legal = engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == shovel));
    assert!(activate.is_some(), "Should be able to activate Graveyard Shovel");

    state = resolve_activated(engine::submit_action(&state, activate.unwrap(), &reg), &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_player(P0).life, 22, "Should gain 2 life for exiling a creature");
}

// ══════════════════════════════════════════════════════════════════
// Paraselene
// ══════════════════════════════════════════════════════════════════

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

/// "Destroy all **enchantments**" — and nothing else. The test above puts two
/// enchantments on an otherwise empty battlefield, so a version that destroyed
/// every permanent would pass it.
#[test]
fn paraselene_leaves_everything_that_is_not_an_enchantment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchantment = named_permanent(&mut state, &reg, "Glorious Anthem", P1);
    let creature = ready_creature(&mut state, P1, 2, 2);
    let land = named_permanent(&mut state, &reg, "Forest", P1);
    let artifact = named_permanent(&mut state, &reg, "Cobbled Wings", P1);

    let spell = castable_spell(&mut state, &reg, "Paraselene", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert_eq!(state.get_object(enchantment).unwrap().zone, Zone::Graveyard);
    for (id, what) in [(creature, "creature"), (land, "land"), (artifact, "artifact")] {
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield,
            "a {what} is not an enchantment");
    }
    assert_eq!(state.get_player(P0).life, 21, "one life, for the one enchantment");
}

/// "You gain 1 life for each enchantment **destroyed this way**" — so an
/// enchantment that survived the destruction is not one of them.
///
/// This is the sentence a naive implementation gets wrong by counting the
/// enchantments it found rather than the ones that died, and the only reason
/// the card needs `try_destroy_all`'s results at all.
#[test]
fn paraselene_gains_no_life_for_an_enchantment_it_could_not_destroy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doomed = named_permanent(&mut state, &reg, "Glorious Anthem", P1);
    let survivor = named_permanent(&mut state, &reg, "Intangible Virtue", P1);
    grant_keyword(&mut state, survivor, Keyword::Indestructible);

    let spell = castable_spell(&mut state, &reg, "Paraselene", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert_eq!(state.get_object(doomed).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(survivor).unwrap().zone, Zone::Battlefield,
        "indestructible answers a mass destroy like any other (CR 702.12b)");
    assert_eq!(state.get_player(P0).life, 21,
        "one life, for the one that was destroyed this way");
}

// ══════════════════════════════════════════════════════════════════
// Into the Maw of Hell
// ══════════════════════════════════════════════════════════════════

/// "Destroy target land. Into the Maw of Hell deals 13 damage to target
/// creature." Both halves, in one cast.
///
/// This section header had no test under it, while the file's own list of
/// covered cards named the card — the rest of its coverage lives in
/// `fizzle.rs` and `characteristics_targeting.rs` and is all about targeting.
/// Nothing anywhere asserted the spell's plain effect.
#[test]
fn into_the_maw_of_hell_destroys_the_land_and_burns_the_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = named_permanent(&mut state, &reg, "Forest", P1);
    let creature = ready_creature(&mut state, P1, 5, 5);
    let maw = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);

    let state = cast_and_resolve(&state, &reg, maw,
        vec![Target::Object(land), Target::Object(creature)]);

    assert_eq!(state.get_object(land).unwrap().zone, Zone::Graveyard,
        "the land is destroyed");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 13,
        "and the creature takes 13, not some other number");
    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::NonCombatDamageDealt { .. })),
        "a sorcery's damage is not combat damage — the distinction is what \
         lifelink and Brimstone Volley's morbid read");
    assert!(state.get_object(creature).unwrap().damaged_by.contains(&maw),
        "and the source is recorded, which is what 'dealt damage by' effects read");
}

/// "**Destroy** target land" goes through the destruction pipeline, so an
/// indestructible land survives — and the other half of the spell happens
/// anyway, because the two halves are independent.
#[test]
fn into_the_maw_of_hell_cannot_destroy_an_indestructible_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = named_permanent(&mut state, &reg, "Forest", P1);
    grant_keyword(&mut state, land, Keyword::Indestructible);
    let creature = ready_creature(&mut state, P1, 5, 5);
    let maw = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);

    let state = cast_and_resolve(&state, &reg, maw,
        vec![Target::Object(land), Target::Object(creature)]);

    assert_eq!(state.get_object(land).unwrap().zone, Zone::Battlefield,
        "'destroy' does not move an indestructible permanent");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 13,
        "the creature still takes its 13");
}

/// Ruling: "Into the Maw of Hell targets both the land and the creature. You
/// can only cast it if you can choose a legal target for both." CR 601.2c —
/// with only one of the two kinds on the battlefield there is no legal set of
/// targets, so the spell is not castable at all.
#[test]
fn into_the_maw_of_hell_needs_a_legal_target_for_both_halves() {
    let reg = registry();

    let mut state = game_at_step(Step::PrecombatMain, P0);
    let maw = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);
    assert!(!can_cast(&state, &reg, maw), "an empty battlefield offers neither");

    let land = named_permanent(&mut state, &reg, "Forest", P1);
    assert!(!can_cast(&state, &reg, maw), "a land alone is not enough");

    state.move_object(land, Zone::Graveyard, &reg);
    ready_creature(&mut state, P1, 5, 5);
    assert!(!can_cast(&state, &reg, maw), "and neither is a creature alone");

    named_permanent(&mut state, &reg, "Forest", P1);
    assert!(can_cast(&state, &reg, maw), "with one of each it is castable");
}

// ══════════════════════════════════════════════════════════════════
// Maw of the Mire
// ══════════════════════════════════════════════════════════════════

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

/// Ruling: "If you only have one card in your graveyard when Make a Wish
/// resolves, that card will be returned to your hand." And with none, it
/// resolves having returned nothing.
#[test]
fn make_a_wish_returns_what_is_there_when_the_graveyard_is_short() {
    for stocked in [0usize, 1] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let cards: Vec<_> = (0..stocked).map(|_| {
            let c = ready_creature(&mut state, P0, 1, 1);
            state.move_object(c, Zone::Graveyard, &reg);
            c
        }).collect();

        let spell = castable_spell(&mut state, &reg, "Make a Wish", P0);
        let state = cast_and_resolve(&state, &reg, spell, vec![]);

        for c in &cards {
            assert_eq!(state.get_object(*c).unwrap().zone, Zone::Hand,
                "{stocked} card(s): what is there comes back");
        }
        assert_eq!(state.get_object(spell).unwrap().zone, Zone::Graveyard,
            "{stocked} card(s): and the spell finishes either way");
    }
}

/// "from **your** graveyard" — a card goes to its owner's (CR 404.3), and an
/// opponent's is not yours.
#[test]
fn make_a_wish_does_not_reach_an_opponents_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let theirs = ready_creature(&mut state, P1, 1, 1);
    state.move_object(theirs, Zone::Graveyard, &reg);
    let mine = ready_creature(&mut state, P0, 1, 1);
    state.move_object(mine, Zone::Graveyard, &reg);

    let spell = castable_spell(&mut state, &reg, "Make a Wish", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "their card stays in their graveyard");
}

/// "**at random**" — with three cards and two returned there are three
/// possible pairs, and the card does not get to pick. Seeded, so this is a
/// statement about the card rather than about the generator.
#[test]
fn make_a_wish_picks_its_two_at_random() {
    let reg = registry();
    let mut seen = std::collections::HashSet::new();

    for seed in 0..20u64 {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.rng_state = seed;
        let cards: Vec<_> = (0..3).map(|_| {
            let c = ready_creature(&mut state, P0, 1, 1);
            state.move_object(c, Zone::Graveyard, &reg);
            c
        }).collect();

        let spell = castable_spell(&mut state, &reg, "Make a Wish", P0);
        let state = cast_and_resolve(&state, &reg, spell, vec![]);

        let returned: Vec<usize> = cards.iter().enumerate()
            .filter(|(_, id)| state.get_object(**id).unwrap().zone == Zone::Hand)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(returned.len(), 2, "seed {seed}: two of the three");
        seen.insert(returned);
    }

    assert!(seen.len() > 1,
        "twenty seeds returned the same pair every time, so the choice is not \
         random: saw {seen:?}");
}

// ══════════════════════════════════════════════════════════════════
// Moonmist
// ══════════════════════════════════════════════════════════════════

// ══════════════════════════════════════════════════════════════════
// Runic Repetition
// ══════════════════════════════════════════════════════════════════

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

/// Put a named card into exile, owned by `owner`.
fn card_in_exile(state: &mut mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry,
                 name: &str, owner: PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap();
    let id = state.create_object(card_id, owner, Zone::Exile, None, None);
    state.get_object_mut(id).unwrap().name = name.into();
    id
}

/// "Return target exiled card **with flashback you own** to your hand." Only
/// the positive case was tested, which an implementation ignoring both
/// restrictions also passes.
#[test]
fn runic_repetition_targets_only_your_own_exiled_flashback_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = card_in_exile(&mut state, &reg, "Think Twice", P0);
    let no_flashback = card_in_exile(&mut state, &reg, "Dissipate", P0);
    let theirs = card_in_exile(&mut state, &reg, "Think Twice", P1);

    let spell = castable_spell(&mut state, &reg, "Runic Repetition", P0);
    let offered = offered_targets(&state, &reg, spell);

    assert!(offered.contains(&mtg_engine::actions::Target::Object(mine)),
        "your own exiled card with flashback; offered {offered:?}");
    assert!(!offered.contains(&mtg_engine::actions::Target::Object(no_flashback)),
        "Dissipate has no flashback, so it is not a legal target");
    assert!(!offered.contains(&mtg_engine::actions::Target::Object(theirs)),
        "and an opponent's exiled Think Twice is not one you own");
}

/// Scryfall ruling (2011-09-22): "An effect that gives flashback to an instant
/// or sorcery card in your graveyard stops applying once that card has left
/// the stack. The card won't have flashback while exiled and can't be the
/// target of Runic Repetition (unless it naturally has flashback)."
///
/// Snapcaster Mage's grant is a `TemporaryEffect::GrantFlashback`, which lives
/// on `until_end_of_turn` and so is still in that list while the card sits in
/// exile the same turn. Pushed directly here rather than played out through
/// Snapcaster and a flashback cast, because what is being tested is which
/// source of "has flashback" the targeting reads, not how the grant got there.
#[test]
fn runic_repetition_ignores_flashback_that_was_only_granted() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let granted = card_in_exile(&mut state, &reg, "Dissipate", P0);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantFlashback {
        target: granted,
        cost: ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue),
                                 ManaSymbol::Colored(Color::Blue)]),
    });

    let spell = castable_spell(&mut state, &reg, "Runic Repetition", P0);
    let offered = offered_targets(&state, &reg, spell);

    assert!(!offered.contains(&mtg_engine::actions::Target::Object(granted)),
        "the grant stopped applying when the card left the stack, so it does \
         not have flashback in exile; offered {offered:?}");

    // The control: a card that naturally has flashback is offered from the
    // same board, so the assertion above is about the grant and not about
    // Runic Repetition finding no targets at all.
    let natural = card_in_exile(&mut state, &reg, "Think Twice", P0);
    let offered = offered_targets(&state, &reg, spell);
    assert!(offered.contains(&mtg_engine::actions::Target::Object(natural)),
        "a card that naturally has flashback still is; offered {offered:?}");
}

// ══════════════════════════════════════════════════════════════════
// Full Moon's Rise
// ══════════════════════════════════════════════════════════════════

/// "Werewolf creatures you control get +1/+0 and have trample."
///
/// The static half of the card, which had no test at all — this file listed
/// Full Moon's Rise in its index and then carried an empty section under the
/// heading. Removing the +1/+0, swapping trample for another keyword, and
/// widening the filter to every creature you control each passed the whole
/// workspace.
///
/// Three creatures, because "my Werewolf got bigger" alone is also true of an
/// enchantment that pumps everything.
#[test]
fn full_moons_rise_buffs_only_werewolves_you_control() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
    let mine_other = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let theirs = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P1);
    assert!(state.has_subtype(mine, "Werewolf", &reg) && state.has_subtype(theirs, "Werewolf", &reg),
        "test precondition: both are Werewolves");

    // Before: a 2/2 with no trample.
    assert_eq!(state.effective_power(mine, &reg), Some(2));
    assert!(!state.has_keyword(mine, Keyword::Trample, &reg));

    let _rise = named_permanent(&mut state, &reg, "Full Moon's Rise", P0);

    assert_eq!(state.effective_power(mine, &reg), Some(3),
        "your Werewolf gets +1/+0");
    assert_eq!(state.effective_toughness(mine, &reg), Some(2),
        "and only power — it is +1/+0, not +1/+1");
    assert!(state.has_keyword(mine, Keyword::Trample, &reg),
        "and trample");

    assert_eq!(state.effective_power(mine_other, &reg), Some(2),
        "a creature of yours that is not a Werewolf gets nothing");
    assert!(!state.has_keyword(mine_other, Keyword::Trample, &reg));

    assert_eq!(state.effective_power(theirs, &reg), Some(2),
        "'you control' — the opponent's Werewolf gets nothing");
    assert!(!state.has_keyword(theirs, Keyword::Trample, &reg));
}

/// Ruling (2011-09-22): "In order to regenerate Werewolves involved in combat,
/// you must sacrifice Full Moon's Rise before combat damage is assigned. This
/// means they will lose the +1/+0 and trample bonuses before combat damage
/// assignment."
///
/// The trade is the whole point of the card: the shields arrive and the buff
/// leaves, because the enchantment that was granting it is gone.
#[test]
fn sacrificing_full_moons_rise_trades_the_buff_for_the_shields() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let werewolf = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
    let rise = named_permanent(&mut state, &reg, "Full Moon's Rise", P0);

    assert_eq!(state.effective_power(werewolf, &reg), Some(3), "buffed while it is out");
    assert!(state.has_keyword(werewolf, Keyword::Trample, &reg));

    activate_via_hooks(&mut state, &reg, rise, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.get_object(werewolf).unwrap().regeneration_shields > 0,
        "the Werewolf is regenerating");
    assert_eq!(state.effective_power(werewolf, &reg), Some(2),
        "and has lost the +1/+0, because the enchantment granting it was sacrificed");
    assert!(!state.has_keyword(werewolf, Keyword::Trample, &reg),
        "and the trample with it");
}

/// "Regenerate all **Werewolf** creatures **you control**." The same two
/// restrictions as the static half, on the other ability — regenerating every
/// creature you control passed the whole workspace.
#[test]
fn full_moons_rise_shields_only_werewolves_you_control() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
    let mine_other = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let theirs = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P1);
    let rise = named_permanent(&mut state, &reg, "Full Moon's Rise", P0);

    activate_via_hooks(&mut state, &reg, rise, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.get_object(mine).unwrap().regeneration_shields > 0,
        "your Werewolf regenerates");
    assert_eq!(state.get_object(mine_other).unwrap().regeneration_shields, 0,
        "your Zombie does not");
    assert_eq!(state.get_object(theirs).unwrap().regeneration_shields, 0,
        "and neither does the opponent's Werewolf");
}

// ══════════════════════════════════════════════════════════════════
// Stony Silence
// ══════════════════════════════════════════════════════════════════

#[test]
fn stony_silence_blocks_artifact_mana_abilities() {
    // Per ruling: "No abilities of artifacts can be activated, including mana abilities."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put Sol Ring on the battlefield (artifact with mana ability).
    let sol_ring = named_permanent(&mut state, &reg, "Sol Ring", P0);

    // Without Stony Silence: Sol Ring's mana ability should be available.
    let actions_before = engine::legal_actions(&state, &reg);
    let has_mana_ability = actions_before.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == sol_ring));
    assert!(has_mana_ability, "Sol Ring mana ability should be available without Stony Silence");

    // Put Stony Silence on the battlefield.
    let _stony = named_permanent(&mut state, &reg, "Stony Silence", P0);

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
    let forest = named_permanent(&mut state, &reg, "Forest", P0);

    // Put Stony Silence on the battlefield.
    let _stony = named_permanent(&mut state, &reg, "Stony Silence", P0);

    // Forest mana ability should still work.
    let actions = engine::legal_actions(&state, &reg);
    let has_forest_mana = actions.actions.iter().any(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == forest));
    assert!(has_forest_mana, "Forest mana ability should NOT be blocked by Stony Silence");
}

// ══════════════════════════════════════════════════════════════════
// Witchbane Orb
// ══════════════════════════════════════════════════════════════════

// -------------------------------------------------------------------------
// Witchbane Orb
// -------------------------------------------------------------------------

/// "When this artifact enters, destroy all Curses attached to you."
///
/// The whole triggered half of the card, and the half with no test at all —
/// every Witchbane Orb test was about hexproof. Both arms in one case, because
/// "the Curse on me died" alone would pass for an Orb that destroyed every
/// Curse on the battlefield.
#[test]
fn witchbane_orb_destroys_the_curses_on_you_and_leaves_the_others() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two on me, cast by the opponent; one the other way round.
    let on_me_a = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P1, P0);
    let on_me_b = attach_curse_to_player(&mut state, &reg, "Curse of Oblivion", P1, P0);
    let on_them = attach_curse_to_player(&mut state, &reg, "Curse of the Bloody Tome", P0, P1);

    let orb = castable_spell(&mut state, &reg, "Witchbane Orb", P0);
    let mut state = cast_onto_stack(&state, &reg, orb, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(on_me_a).unwrap().zone, Zone::Graveyard,
        "a Curse attached to you is destroyed whoever controls it");
    assert_eq!(state.get_object(on_me_b).unwrap().zone, Zone::Graveyard,
        "'all Curses', not one of them");
    assert_eq!(state.get_object(on_them).unwrap().zone, Zone::Battlefield,
        "'attached to you' — a Curse on the opponent is untouched, even one \
         you control yourself");
}

/// "**Destroy** all Curses" — destroy, not exile and not sacrifice, so an
/// indestructible Curse stays put (CR 701.7b via 702.12b).
#[test]
fn witchbane_orb_cannot_destroy_an_indestructible_curse() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P1, P0);
    grant_keyword(&mut state, curse, Keyword::Indestructible);

    let orb = castable_spell(&mut state, &reg, "Witchbane Orb", P0);
    let mut state = cast_onto_stack(&state, &reg, orb, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(curse).unwrap().zone, Zone::Battlefield,
        "'destroy' does not move an indestructible permanent");
}

/// CR 113.6: a permanent's static ability functions only while it is on the
/// battlefield. An Orb in the graveyard grants nothing.
#[test]
fn witchbane_orbs_hexproof_stops_when_it_leaves_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);
    assert!(state.player_has_hexproof(P0, &reg), "test precondition");

    state.move_object(orb, Zone::Graveyard, &reg);
    assert!(!state.player_has_hexproof(P0, &reg),
        "the static ability functions only on the battlefield");
}

/// Player has hexproof when they control Witchbane Orb.
#[test]
fn grants_player_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    assert!(!state.player_has_hexproof(P0, &reg), "Should not have hexproof without Orb");

    let _orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    assert!(state.player_has_hexproof(P0, &reg), "Should have hexproof with Orb");
    assert!(!state.player_has_hexproof(P1, &reg), "Opponent should not have hexproof");
}

/// Opponent cannot target a player with hexproof.
#[test]
fn opponent_cannot_target_hexproof_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);

    // P0 has Witchbane Orb.
    let _orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    // P1 tries to cast a player-targeting spell at P0.
    // Bump in the Night: {B} Sorcery — "Target opponent loses 3 life."
    let bump = castable_spell(&mut state, &reg, "Bump in the Night", P1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let cast_at_p0: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, targets, .. } = a {
            object_id == &bump && targets.iter().any(|t| {
                matches!(t, mtg_engine::actions::Target::Player(p) if *p == P0)
            })
        } else {
            false
        }
    }).collect();

    assert!(cast_at_p0.is_empty(),
        "Should not be able to target hexproof player");
}

/// Player can still target themselves even with hexproof (hexproof only prevents opponents).
#[test]
fn can_target_self_with_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has Witchbane Orb.
    let _orb = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    // P0 tries to cast Dream Twist (targets any player) at themselves.
    let twist = castable_spell(&mut state, &reg, "Dream Twist", P0);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let cast_at_p0: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, targets, .. } = a {
            object_id == &twist && targets.iter().any(|t| {
                matches!(t, mtg_engine::actions::Target::Player(p) if *p == P0)
            })
        } else {
            false
        }
    }).collect();

    assert!(!cast_at_p0.is_empty(),
        "Player should be able to target themselves even with hexproof");
}

// -------------------------------------------------------------------------
// Kessig Wolf Run
// -------------------------------------------------------------------------

/// Can activate with {R}{G} and X=0 (minimum).
#[test]
fn can_activate_with_rg_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let _creature = ready_creature(&mut state, P0, 2, 2);

    // Add just {R}{G} — X will be 0.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_activate = actions.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { ability_index: 1, .. })
    });

    assert!(can_activate,
        "Should be able to activate with just RG (X=0)");
}

/// Cannot activate without at least {R}{G}.
#[test]
fn cannot_activate_without_rg() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let _creature = ready_creature(&mut state, P0, 2, 2);

    // Only {R} — no green.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_activate = actions.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { ability_index: 1, .. })
    });

    assert!(!can_activate,
        "Should not be able to activate without both R and G");
}

/// X=3 with 5 mana gives +3/+0 and trample.
#[test]
fn x_equals_3_gives_plus_3() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Add {R}{G} + 3 colorless = 5 mana total. We'll fund X = 3 from the
    // remaining 3 colorless after the {R}{G} non-X portion is paid.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let action = Action::ActivateAbility {
        object_id: wolf_run,
        ability_index: 1,
        targets: vec![mtg_engine::actions::Target::Object(creature)],
        tap_plan: vec![],
        sacrifice: None,
        x_value: None,
        source_card_id: None,
    };

    state = resolve_activated(mtg_engine::engine::submit_action(&state, &action, &reg), &reg);
    state = resolve_funding_max(&state, &reg);

    // Check +3/+0 effect.
    let power = state.effective_power(creature, &reg).unwrap_or(0);
    assert_eq!(power, 5,
        "Creature should have 2 + 3 = 5 power (got {power})");

    // Asked through the accessor, not by finding the `until_end_of_turn`
    // entry: the entry existing and the engine honouring it are two different
    // claims, and only the second is what the card promises.
    assert!(state.has_keyword(creature, Keyword::Trample, &reg),
        "Creature should have trample");

    // All mana should be spent.
    assert!(state.get_player(P0).mana_pool.is_empty(),
        "All mana should be spent");
}

/// X=0 gives just trample (no power boost).
#[test]
fn x_equals_0_gives_trample_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Just {R}{G} — X = 0.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let action = Action::ActivateAbility {
        object_id: wolf_run,
        ability_index: 1,
        targets: vec![mtg_engine::actions::Target::Object(creature)],
        tap_plan: vec![],
        sacrifice: None,
        x_value: None,
        source_card_id: None,
    };

    state = resolve_activated(mtg_engine::engine::submit_action(&state, &action, &reg), &reg);

    // Power should remain 2 (X=0 gives +0/+0).
    let power = state.effective_power(creature, &reg).unwrap_or(0);
    assert_eq!(power, 2, "Creature should still have 2 power with X=0");

    // Should still have trample.
    assert!(state.has_keyword(creature, Keyword::Trample, &reg),
        "Creature should have trample even with X=0");
}

/// "**Target creature**" — no "you control". An opponent's creature is a legal
/// target, which is how the card is used to push a blocker through or to make
/// an opponent's creature trample over its own team in a fight it will lose.
#[test]
fn kessig_wolf_run_can_pump_an_opponents_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let theirs = ready_creature(&mut state, P1, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let offered = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .any(|a| matches!(a, Action::ActivateAbility { object_id, ability_index: 1, targets, .. }
            if object_id == wolf_run && targets == vec![Target::Object(theirs)]));
    assert!(offered, "an opponent's creature is a legal target");
}

/// CR 608.2b: a target that has left the battlefield is illegal, and the
/// ability is countered by game rules — no pump, no trample.
#[test]
fn kessig_wolf_run_does_nothing_when_its_target_is_gone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let action = Action::ActivateAbility {
        object_id: wolf_run,
        ability_index: 1,
        targets: vec![Target::Object(creature)],
        tap_plan: vec![],
        sacrifice: None,
        x_value: None,
        source_card_id: None,
    };
    let mut state = mtg_engine::engine::submit_action(&state, &action, &reg);

    // In response, the creature dies.
    state.move_object(creature, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::ModifyPT { target, .. }
        | mtg_engine::state::TemporaryEffect::GrantKeyword { target, .. }
        if *target == creature)),
        "the ability is countered, so neither the pump nor the trample is \
         applied to a creature that is no longer there");
}


/// The Assistant is the pool's only mana source that costs you something to
/// use, which is what `ManaAbilityDef::has_side_effects` is for: the auto-tap
/// planner ranks it last (`ManaSourceKind::HasSideEffects`), below even a
/// creature that merely loses the chance to attack.
///
/// With a Forest beside it, casting a {1} spell must not mill.
#[test]
fn autotap_taps_a_land_before_it_mills_you() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest = named_permanent(&mut state, &reg, "Forest", P0);
    let assistant = named_permanent(&mut state, &reg, "Deranged Assistant", P0);
    let lib_card = state.create_object(
        reg.get_id_by_name("Forest").unwrap(), P0, Zone::Library, None, None);
    state.players[0].library_order = vec![lib_card];

    // Blazing Torch costs {1} — generic, so either source could pay it.
    let torch = spell_in_hand(&mut state, &reg, "Blazing Torch", P0);
    let legal = engine::legal_actions(&state, &reg);
    let cast = legal.actions.iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == torch))
        .expect("Blazing Torch should be castable off either source")
        .clone();
    let state = engine::submit_action(&state, &cast, &reg);

    assert!(state.get_object(forest).unwrap().tapped, "the Forest paid for it");
    assert!(!state.get_object(assistant).unwrap().tapped,
        "not the Assistant, whose mana costs a card");
    assert_eq!(state.get_object(lib_card).unwrap().zone, Zone::Library,
        "so nothing was milled");
}

/// The side-effect tier is not just "below a land" — it is below another
/// creature's mana too, and it outranks the colour-demand tiebreak that would
/// otherwise spare the Pilgrim's {W} for the white card in hand.
///
/// This is what `has_side_effects` buys. Without it the Assistant is ranked as
/// an ordinary `Creature` alongside Avacyn's Pilgrim, and the demand for {W}
/// picks the Assistant — milling a card to keep a colour open.
#[test]
fn autotap_would_rather_lose_a_colour_than_mill_a_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let assistant = named_permanent(&mut state, &reg, "Deranged Assistant", P0);
    let lib_card = state.create_object(
        reg.get_id_by_name("Forest").unwrap(), P0, Zone::Library, None, None);
    state.players[0].library_order = vec![lib_card];

    // A white card in hand, so the Pilgrim's {W} is the mana in demand.
    spell_in_hand(&mut state, &reg, "Doomed Traveler", P0);
    let torch = spell_in_hand(&mut state, &reg, "Blazing Torch", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cast = legal.actions.iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == torch))
        .expect("Blazing Torch should be castable off either creature")
        .clone();
    let state = engine::submit_action(&state, &cast, &reg);

    assert!(state.get_object(pilgrim).unwrap().tapped,
        "the Pilgrim pays, even though its {{W}} is wanted for the card in hand");
    assert!(!state.get_object(assistant).unwrap().tapped,
        "the Assistant is ranked below it — its mana costs a card");
    assert_eq!(state.get_object(lib_card).unwrap().zone, Zone::Library,
        "so nothing was milled");
}

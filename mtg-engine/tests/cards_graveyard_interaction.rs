//! Cards that read, return from, or exile a graveyard.
//!
//! Cards covered (10), so this is greppable by name as well as by rule:
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
use mtg_engine::cards::CardRegistry;
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

/// Put a Ghoulraiser onto the battlefield and let its enters trigger resolve.
fn ghoulraiser_enters(state: &mut mtg_engine::state::GameState,
                      reg: &mtg_engine::cards::CardRegistry) -> ObjectId {
    let raiser = named_permanent(state, reg, "Ghoulraiser", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: raiser, controller: P0,
    });
    triggers::process_triggers(state, reg);
    raiser
}

/// "return a **Zombie card**" — the restriction, in both of its parts. The
/// existing test puts only a Zombie in the graveyard, so an implementation
/// that returned any card at all passes it.
#[test]
fn ghoulraiser_returns_only_a_zombie_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let not_a_zombie = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    // CR 109.1: a token in a graveyard is not a card, however Zombie it is.
    let token = *state.create_token_with_subtypes(
        "Zombie Token", P0, 2, 2, vec![Color::Black], vec![CardType::Creature],
        vec![], vec!["Zombie".into()], &reg)
        .first().expect("token created");
    state.move_object(token, Zone::Graveyard, &reg);

    ghoulraiser_enters(&mut state, &reg);

    assert_eq!(state.get_object(not_a_zombie).unwrap().zone, Zone::Graveyard,
        "a Bear is not a Zombie card");
    assert_eq!(state.get_object(token).map(|o| o.zone), Some(Zone::Graveyard),
        "and a Zombie token is not a Zombie *card*");
    assert!(state.objects_in_zone(Zone::Hand, P0).is_empty(),
        "so nothing came back at all");
}

/// A Ghoulraiser that died with its own trigger on the stack is itself a
/// Zombie card in that graveyard, so it is one of the candidates. The card's
/// comment says so; nothing checked it — the removal-in-response test has a
/// second Zombie sitting there to be found instead.
#[test]
fn a_dead_ghoulraiser_can_return_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let raiser = named_permanent(&mut state, &reg, "Ghoulraiser", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: raiser, controller: P0,
    });
    triggers::collect_triggers(&mut state, &reg);
    // Removal resolves first: the Ghoulraiser is in the graveyard, and it is
    // the only Zombie card there.
    state.move_object(raiser, Zone::Graveyard, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(raiser).unwrap().zone, Zone::Hand,
        "the only Zombie card in the graveyard was the Ghoulraiser itself");
}

/// "at random" — with more than one candidate the choice has to vary. The
/// suite-wide guard only checks that this card reaches an RNG at all.
#[test]
fn ghoulraiser_picks_at_random_among_several_zombies() {
    let reg = registry();
    let mut seen = std::collections::HashSet::new();

    for _ in 0..40 {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let candidates: Vec<ObjectId> = ["Walking Corpse", "Diregraf Ghoul", "Makeshift Mauler"]
            .iter()
            .map(|n| named_card_in_graveyard(&mut state, &reg, n, P0))
            .collect();

        ghoulraiser_enters(&mut state, &reg);

        let returned: Vec<ObjectId> = candidates.iter().copied()
            .filter(|c| state.get_object(*c).unwrap().zone == Zone::Hand)
            .collect();
        assert_eq!(returned.len(), 1, "exactly one card comes back, not all of them");
        seen.insert(state.get_object(returned[0]).unwrap().name.clone());
    }

    assert!(seen.len() > 1,
        "40 entries always returned the same Zombie, so the choice is not \
         random; saw {seen:?}");
}

/// An empty graveyard, and one with no Zombie cards in it, both mean the
/// trigger does as much as it can — which is nothing — without erroring.
#[test]
fn ghoulraiser_with_nothing_to_return_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let raiser = ghoulraiser_enters(&mut state, &reg);

    assert_eq!(state.get_object(raiser).unwrap().zone, Zone::Battlefield);
    assert!(state.objects_in_zone(Zone::Hand, P0).is_empty(),
        "an empty graveyard returns nothing");
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

        // CR 701.19b: the find is offered, not taken — even with one basic.
        state = mtg_engine::engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: ResolvedChoice::ChosenTarget(Some(Target::Object(forest))),
            },
            &reg,
        );

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

/// CR 701.19b: "If a player is searching a hidden zone for cards with stated
/// characteristics ... that player isn't required to find some or all of those
/// cards even if they're present in that zone."
///
/// So the one basic land in the library is offered, not taken — and CR 701.19a
/// still shuffles, because the search happened either way.
#[test]
fn caravan_vigil_may_search_and_find_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A library with one basic land and enough other cards that a shuffle is
    // visible in the order.
    let forest = stock_library(&mut state, &reg, P0, 1)[0];
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..8 {
        let id = state.create_object(bears_id, P0, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P0).library_order.push(id);
    }

    let vigil = castable_spell(&mut state, &reg, "Caravan Vigil", P0);
    let state = cast_and_resolve(&state, &reg, vigil, vec![]);

    let before: Vec<_> = state.get_player(P0).library_order.clone();
    assert!(state.awaiting_action.is_some(),
        "the only basic land is offered, not taken for the player");

    // Decline it.
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) },
        &reg,
    );

    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Library,
        "declining leaves the land in the library");
    assert!(state.awaiting_action.is_none(),
        "and there is no morbid question, because nothing was found");
    assert_ne!(state.get_player(P0).library_order, before,
        "CR 701.19a: the search happened, so the library is shuffled anyway");
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

/// "Exile target **card** from **a** graveyard" — any card type, in anybody's
/// graveyard, the caster's own included. The test below takes an opponent's
/// creature card, which is also what "exile target creature card from an
/// opponent's graveyard" would allow; this one takes the caster's own land.
#[test]
fn purify_the_grave_exiles_any_card_from_any_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own_land = named_card_in_graveyard(&mut state, &reg, "Forest", P0);
    let their_creature = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);

    let purify = castable_spell(&mut state, &reg, "Purify the Grave", P0);

    // The offer is where the scope lives — the cast handler takes the targets
    // it is given, so resolving one proves only that the card exiles what it
    // was pointed at.
    let offers: Vec<Vec<Target>> = mtg_engine::engine::legal_actions(&state, &reg).actions
        .into_iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if object_id == purify => Some(targets),
            _ => None,
        })
        .collect();
    assert!(offers.contains(&vec![Target::Object(own_land)]),
        "a land in the caster's own graveyard is offered: \"a graveyard\", and \
         \"card\" rather than \"creature card\". Offered: {offers:?}");
    assert!(offers.contains(&vec![Target::Object(their_creature)]),
        "and so is the opponent's creature card");

    let new_state = cast_and_resolve(&state, &reg, purify, vec![Target::Object(own_land)]);
    assert_eq!(new_state.get_object(own_land).unwrap().zone, Zone::Exile);
}

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

/// "Put target creature on top of its **owner's** library." Two words carry the
/// whole card, so the setup makes both of them wrong to get right: the library
/// already has cards in it (an empty one puts top and bottom in the same
/// place), and the creature is owned by one player while another controls it —
/// a stolen creature goes home, not to the thief's library.
#[test]
fn grasp_of_phantoms_puts_creature_on_top_of_its_owners_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let already_there = stock_library(&mut state, &reg, P1, 2);
    let thiefs_library = stock_library(&mut state, &reg, P0, 2);

    // Owned by P1, controlled by P0 — Grasp is cast by the controller.
    let target_creature = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.get_object_mut(target_creature).unwrap().controller = P0;

    let grasp = castable_spell(&mut state, &reg, "Grasp of Phantoms", P0);
    let new_state = cast_and_resolve(&state, &reg, grasp, vec![Target::Object(target_creature)]);

    assert_eq!(new_state.get_object(target_creature).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_player(P1).library_order,
        vec![target_creature, already_there[0], already_there[1]],
        "on top of the owner's library, above what was already in it");
    assert_eq!(new_state.get_player(P0).library_order, thiefs_library,
        "and the controller's library is untouched");
}

/// A token put on top of a library ceases to exist (CR 111.7, SBA 704.5d), so
/// nothing arrives there: the library is no deeper than it was, and the next
/// draw is the real card that was already on top. The library's order is a
/// list of object ids kept alongside the zone, so a vanished token left listed
/// there is a card that can be drawn and isn't — the draw comes up empty, the
/// hand doesn't grow, and a player who should have decked out doesn't.
#[test]
fn grasp_of_phantoms_puts_no_phantom_on_the_library_when_it_bounces_a_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let real_card = stock_library(&mut state, &reg, P1, 1)[0];
    let token = state.create_token_with_subtypes(
        "", P1, 1, 1,
        vec![Color::White], vec![CardType::Creature], vec![], vec!["Spirit".into()],
        &reg,
    )[0];

    let grasp = castable_spell(&mut state, &reg, "Grasp of Phantoms", P0);
    let mut state = cast_and_resolve(&state, &reg, grasp, vec![Target::Object(token)]);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert!(state.get_object(token).is_none(), "the token ceased to exist");
    assert_eq!(state.get_player(P1).library_order, vec![real_card],
        "the library holds the one real card, with no phantom on top of it");
    assert_eq!(state.get_player_mut(P1).draw_top_card(), Some(real_card),
        "so the next draw is that real card");
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

/// Ruling: "If you target yourself with this spell, you must reveal your
/// entire hand to the other players just as any other player would."
///
/// "Target player" carries no restriction, so you are a legal target — and
/// "you choose" still means the spell's controller, which here is the same
/// person. Exiling a card out of your own hand is a real, if unusual, line.
#[test]
fn night_terrors_can_target_its_own_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let forest = spell_in_hand(&mut state, &reg, "Forest", P0);
    let terrors = castable_spell(&mut state, &reg, "Night Terrors", P0);

    assert!(mtg_engine::engine::legal_actions(&state, &reg).actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, targets, .. }
            if *object_id == terrors && targets == &[Target::Player(P0)])),
        "you are a legal target for your own Night Terrors");

    let state = cast_and_resolve(&state, &reg, terrors, vec![Target::Player(P0)]);

    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Exile,
        "the nonland card leaves your own hand");
    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Hand,
        "and the land is still not a legal choice");
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

/// "return a creature card **at random** from your graveyard to your hand."
///
/// A card that always returned the first eligible creature would satisfy every
/// other Woodland Sleuth test — they each set up exactly one candidate — so
/// this is the only place the randomness itself is under test. Sixty draws
/// from three candidates: a fixed choice fails with certainty, a uniform one
/// fails with probability 3 * (1/3)^60.
#[test]
fn woodland_sleuth_returns_a_random_creature_not_a_fixed_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let candidates: Vec<_> = ["Grizzly Bears", "Makeshift Mauler", "Stitched Drake"]
        .iter()
        .map(|n| named_card_in_graveyard(&mut state, &reg, n, P0))
        .collect();
    let sleuth = named_permanent(&mut state, &reg, "Woodland Sleuth", P0);
    let behavior = reg.get(state.get_object(sleuth).unwrap().card_id).unwrap();

    let mut seen = std::collections::HashSet::new();
    for _ in 0..60 {
        behavior.on_enter_battlefield(&mut state, sleuth, &[], &reg);
        let returned = candidates.iter().copied()
            .find(|&id| state.get_object(id).unwrap().zone == Zone::Hand)
            .expect("one of the three creature cards is returned every time");
        seen.insert(returned);
        // Put it back so every draw sees the same three candidates.
        state.move_object(returned, Zone::Graveyard, &reg);
    }

    assert!(seen.len() > 1,
        "sixty resolutions over three creature cards only ever returned {} of \
         them — the choice is not being made at random", seen.len());
}

/// "from **your** graveyard" — CR 404.3 puts a card in its owner's graveyard,
/// so an opponent's creature card is not a legal choice no matter who controls
/// the Sleuth.
#[test]
fn woodland_sleuth_does_not_reach_into_an_opponents_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let theirs = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let sleuth = named_permanent(&mut state, &reg, "Woodland Sleuth", P0);
    reg.get(state.get_object(sleuth).unwrap().card_id).unwrap()
        .on_enter_battlefield(&mut state, sleuth, &[], &reg);

    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "the only creature card in the game is in the opponent's graveyard, \
         which is not \"your graveyard\"");
}

// -------------------------------------------------------------------------
// Ghoulcaller's Chant
// -------------------------------------------------------------------------

/// The offered target sets, split by arity: (mode-1 singles, mode-2 pairs).
fn modes(
    state: &mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    chant: ObjectId,
) -> (Vec<Target>, Vec<Vec<Target>>) {
    let sets = offered_target_sets(state, reg, chant);
    let singles = sets.iter().filter(|t| t.len() == 1).map(|t| t[0].clone()).collect();
    let pairs = sets.into_iter().filter(|t| t.len() == 2).collect();
    (singles, pairs)
}

/// Which cards each mode may name, for every shape of graveyard that matters.
///
/// The negative half of each row is as important as the positive: mode 2 needs
/// *two* Zombies, so one Zombie beside a Bear offers no pair, and two Bears
/// offer no pair either — an engine that ignored the Zombie restriction would
/// pass a test that only looked at the all-Zombie case.
#[test]
fn each_mode_offers_exactly_the_cards_it_may_name() {
    // (cards in your graveyard, cards in the opponent's, mode-1 count, mode-2 count)
    const CASES: &[(&[&str], &[&str], usize, usize, &str)] = &[
        (&["Grizzly Bears"], &[], 1, 0,
         "one non-Zombie: mode 1 only"),
        (&["Walking Corpse", "Diregraf Ghoul"], &[], 2, 1,
         "two Zombies: either one alone, or both together"),
        (&["Grizzly Bears", "Savannah Lions"], &[], 2, 0,
         "two non-Zombies: no pair, because mode 2 names Zombies"),
        (&["Walking Corpse", "Grizzly Bears"], &[], 2, 0,
         "one Zombie and one not: still no pair"),
        (&["Grizzly Bears", "Walking Corpse", "Diregraf Ghoul"], &[], 3, 1,
         "three cards, two of them Zombies: three singles and the one pair"),
        (&[], &["Grizzly Bears"], 0, 0,
         "'your graveyard' — an opponent's creature card is not a legal target"),
        // Mode 2's half of the same rule. The row above uses a non-Zombie, so
        // it is equally explained by the Zombie restriction; only an
        // opponent's *Zombie* separates "not yours" from "not a Zombie".
        (&[], &["Walking Corpse", "Diregraf Ghoul"], 0, 0,
         "two Zombies in the opponent's graveyard are still not yours"),
        (&["Walking Corpse"], &["Diregraf Ghoul"], 1, 0,
         "mode 2 cannot make up its pair from an opponent's Zombie"),
    ];

    for &(mine, theirs, singles_expected, pairs_expected, why) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let ids: Vec<ObjectId> = mine.iter()
            .map(|n| named_card_in_graveyard(&mut state, &reg, n, P0))
            .collect();
        for n in theirs {
            named_card_in_graveyard(&mut state, &reg, n, P1);
        }

        let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
        let (singles, pairs) = modes(&state, &reg, chant);

        assert_eq!(singles.len(), singles_expected, "{why}: mode 1 count");
        assert_eq!(pairs.len(), pairs_expected, "{why}: mode 2 count");

        // Every card offered is one of yours, and every one of yours is offered.
        for id in &ids {
            assert!(singles.contains(&Target::Object(*id)),
                "{why}: every creature card in your graveyard is a mode-1 target");
        }
        for pair in &pairs {
            for t in pair {
                let Target::Object(id) = t else { panic!("{why}: mode 2 names cards") };
                assert!(state.has_subtype(*id, "Zombie", &reg),
                    "{why}: mode 2 names Zombies only");
            }
        }
    }
}

/// Mode 1 resolving: the named card comes back.
#[test]
fn mode_one_returns_the_creature_card_it_named() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let bystander = named_card_in_graveyard(&mut state, &reg, "Savannah Lions", P0);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let state = cast_and_resolve(&state, &reg, chant, vec![Target::Object(bear)]);

    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(bystander).unwrap().zone, Zone::Graveyard,
        "only the card it named");
}

/// Mode 2 resolving: both named Zombies come back.
#[test]
fn mode_two_returns_both_zombies_it_named() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let a = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let b = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let state = cast_and_resolve(&state, &reg, chant,
        vec![Target::Object(a), Target::Object(b)]);

    assert_eq!(state.get_object(a).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(b).unwrap().zone, Zone::Hand);
}

/// CR 608.2b: mode 2 names two targets, so one of them leaving the graveyard
/// in response does not stop the other coming back. The spell is countered
/// only if *both* are gone.
///
/// This is what the zone guard in the card's `on_resolve` loop is for — with
/// one target it would be unreachable, because a spell whose only target is
/// illegal never resolves at all.
#[test]
fn mode_two_returns_the_zombie_that_is_still_there() {
    let reg = registry();

    // One of the two leaves: the other still comes back.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let b = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let mut state = cast_onto_stack(&state, &reg, chant,
        vec![Target::Object(a), Target::Object(b)]);
    state.move_object(a, Zone::Exile, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(b).unwrap().zone, Zone::Hand,
        "the Zombie still in the graveyard comes back");
    assert_eq!(state.get_object(a).unwrap().zone, Zone::Exile,
        "and the one that left is not dragged out of exile");

    // Both leave: countered by game rules, so neither moves.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let b = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let mut state = cast_onto_stack(&state, &reg, chant,
        vec![Target::Object(a), Target::Object(b)]);
    state.move_object(a, Zone::Exile, &reg);
    state.move_object(b, Zone::Exile, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(chant).unwrap().zone, Zone::Graveyard,
        "no legal target left, so the spell is countered by game rules");
    assert_eq!(state.get_object(a).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(b).unwrap().zone, Zone::Exile);
}

// -------------------------------------------------------------------------
// Memory's Journey
// -------------------------------------------------------------------------

/// Shuffles a card from your own graveyard into your library.
#[test]
fn shuffles_own_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P0), Target::Object(card)]);

    assert_eq!(new_state.get_object(card).unwrap().zone, Zone::Library,
        "Card should be shuffled into library");
}

/// Shuffles a card from opponent's graveyard into their library.
#[test]
fn shuffles_opponent_graveyard_card_into_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P1), Target::Object(card)]);

    assert_eq!(new_state.get_object(card).unwrap().zone, Zone::Library,
        "Opponent's card should be shuffled into their library");
}

/// Can target up to 3 cards from the same graveyard.
#[test]
fn shuffles_up_to_three_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card1 = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let card2 = named_card_in_graveyard(&mut state, &reg, "Doom Blade", P0);
    let card3 = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![
        Target::Player(P0),
        Target::Object(card1),
        Target::Object(card2),
        Target::Object(card3),
    ]);

    assert_eq!(new_state.get_object(card1).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(card2).unwrap().zone, Zone::Library);
    assert_eq!(new_state.get_object(card3).unwrap().zone, Zone::Library);
}

/// Resolving against one player's graveyard leaves the other's alone.
/// (Which cards are *offered* is checked in `multi_target_and_mill.rs`, on
/// `legal_actions`; this one is about what resolution actually moves.)
#[test]
fn resolving_only_moves_the_targeted_players_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own_card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let opp_card = named_card_in_graveyard(&mut state, &reg, "Doom Blade", P1);

    // Memory's Journey targeting P0's graveyard card.
    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let new_state = cast_and_resolve(&state, &reg, journey, vec![Target::Player(P0), Target::Object(own_card)]);

    // P0's card should be in library, P1's card should still be in graveyard.
    assert_eq!(new_state.get_object(own_card).unwrap().zone, Zone::Library,
        "Own card should be shuffled into library");
    assert_eq!(new_state.get_object(opp_card).unwrap().zone, Zone::Graveyard,
        "Opponent's card should remain in their graveyard");
}

/// Ruling (2011-09-22): "Any of the targeted cards that are illegal targets by
/// the time Memory's Journey resolves aren't shuffled into their owner's
/// library."
///
/// Letting a card that has left the graveyard be moved anyway passed the whole
/// workspace: every test resolves against cards that stayed put.
#[test]
fn memorys_journey_skips_a_card_that_left_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let stays = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let leaves = named_card_in_graveyard(&mut state, &reg, "Doom Blade", P0);

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let mut state = cast_onto_stack(&state, &reg, journey,
        vec![Target::Player(P0), Target::Object(stays), Target::Object(leaves)]);

    // One of the targeted cards is exiled in response.
    state.move_object(leaves, Zone::Exile, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(stays).unwrap().zone, Zone::Library,
        "the card still in the graveyard is shuffled in");
    assert_eq!(state.get_object(leaves).unwrap().zone, Zone::Exile,
        "the one that left is not a legal target any more, so it stays where it is");
    assert!(!state.get_player(P0).library_order.contains(&leaves),
        "and it is not in the library either");
}

/// Ruling (2011-09-22): "If no cards were targeted by Memory's Journey or if
/// all the targeted cards are illegal targets by the time Memory's Journey
/// resolves, the targeted player will still shuffle their library."
///
/// "Up to three" includes none, so the spell resolves on the player alone —
/// it does not fizzle for want of a card. Skipping the shuffle when nothing
/// moved passed the whole workspace.
///
/// The shuffle itself is not observable while the RNG is unseeded; what this
/// pins down is that the spell resolves rather than being countered by game
/// rules, and that it leaves the library's contents alone.
#[test]
fn memorys_journey_resolves_on_the_player_alone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let mut library = Vec::new();
    for _ in 0..4 {
        let c = state.create_object(card_id, P0, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P0).library_order.push(c);
        library.push(c);
    }

    let journey = castable_spell(&mut state, &reg, "Memory's Journey", P0);
    let mut state = cast_onto_stack(&state, &reg, journey, vec![Target::Player(P0)]);
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::SpellResolved { object } if *object == journey)),
        "'up to three' includes none, so the spell resolves on the player alone");

    let mut after = state.get_player(P0).library_order.clone();
    after.sort_by_key(|o| o.0);
    library.sort_by_key(|o| o.0);
    assert_eq!(after, library,
        "the library holds exactly what it held — a shuffle moves cards around, \
         it does not add or remove any");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// "Target player shuffles up to three target cards from THEIR graveyard" —
/// the player is a target in its own right, and the cards are constrained by
/// it. Without the player slot the card offered every graveyard at once.
#[test]
fn memorys_journey_targets_a_player_and_then_their_cards() {
    use mtg_engine::cards::TargetRequirement;

    let registry = CardRegistry::with_all_cards();
    let behavior = registry
        .get(registry.get_id_by_name("Memory's Journey").unwrap())
        .unwrap();

    // Matched structurally rather than by searching the Debug string, so a
    // requirement that merely mentions a player somewhere cannot satisfy it.
    match behavior.target_requirement() {
        TargetRequirement::TwoTargets(first, second) => {
            assert!(matches!(*first, TargetRequirement::PlayerOnly),
                "the first target is the player whose graveyard it is, got {first:?}");
            match *second {
                TargetRequirement::UpToTargets(3, inner) => assert!(
                    matches!(*inner, TargetRequirement::GraveyardCardOwnedByTargetPlayer),
                    "the cards must be constrained to the targeted player's \
                     graveyard (CR 601.2c), got {inner:?}"),
                other => panic!("expected 'up to three' cards, got {other:?}"),
            }
        }
        other => panic!("expected TwoTargets(player, up-to-three cards), got {other:?}"),
    }
}

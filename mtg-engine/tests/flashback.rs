//! Tests for the flashback mechanic and the cards that print it.
//!
//! Flashback allows casting a spell from the graveyard for an alternative cost.
//! After resolution (or countering), the spell is exiled instead of returning to graveyard.
//!
//! [`every_flashback_card_is_offered_from_the_graveyard`] sweeps the whole
//! pool, so a new flashback card is covered on the day it is added; the tests
//! below it are for behaviour a sweep cannot reach.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

// ── System tests: flashback mechanics ──────────────────────────────

/// Every card that declares a flashback cost can actually be cast from the
/// graveyard for it (CR 702.33a).
///
/// The cost itself is pinned against the printed text by
/// `card_data_invariants.rs`; what that cannot see is whether the engine ever
/// offers the cast. This does the other half for the whole pool at once — a
/// card whose flashback is unreachable, for whatever reason, is a card whose
/// second half does not exist.
///
/// The pool is deliberately not listed here. Naming the cards is how the
/// module doc came to claim "all 15 flashback cards" while there were 27 of
/// them, seventeen of which no test cast from a graveyard.
#[test]
fn every_flashback_card_is_offered_from_the_graveyard() {
    let reg = registry();
    let mut unreachable = Vec::new();
    let mut checked = 0;

    for name in reg.all_names() {
        let Some(card_id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(card_id) else { continue };
        if data.flashback_cost.is_none() {
            continue;
        }
        checked += 1;

        let mut state = game_at_step(Step::PrecombatMain, P0);
        // One of everything a flashback spell in this set can ask to target,
        // on both sides, so "not offered" means the engine, not the board:
        // a creature, a land and an artifact on the battlefield, a creature
        // card in the graveyard (Unburial Rites), and a library to read.
        for player in [P0, P1] {
            named_permanent(&mut state, &reg, "Grizzly Bears", player);
            named_permanent(&mut state, &reg, "Forest", player);
            named_permanent(&mut state, &reg, "Blazing Torch", player);
            named_card_in_graveyard(&mut state, &reg, "Walking Corpse", player);
            stock_library(&mut state, &reg, player, 5);
        }
        // Enough of every colour to pay any flashback cost in the set several
        // times over.
        add_mana(&mut state, P0, &[
            (ManaType::White, 12), (ManaType::Blue, 12), (ManaType::Black, 12),
            (ManaType::Red, 12), (ManaType::Green, 12), (ManaType::Colorless, 12),
        ]);

        let card = state.create_object(card_id, P0, Zone::Graveyard, None, None);
        state.get_object_mut(card).unwrap().name = name.to_string();

        if !can_cast(&state, &reg, card) {
            unreachable.push(format!("{name}: flashback {}",
                data.flashback_cost.as_ref().map_or(String::new(), std::string::ToString::to_string)));
        }
    }

    assert!(unreachable.is_empty(),
        "{} card(s) print a flashback cost the engine never offers:\n  {}",
        unreachable.len(), unreachable.join("\n  "));
    assert!(checked >= 27, "expected the set's flashback cards; swept only {checked}");
}

/// Flashback is offered when a card with `flashback_cost` is in the graveyard
/// and the player has enough mana.
#[test]
fn flashback_offered_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Geistflame").unwrap();
    let card = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(card).unwrap().name = "Geistflame".into();

    // Geistflame flashback cost is {3}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 4);

    let has_flashback_cast = can_cast(&state, &reg, card);
    assert!(has_flashback_cast,
        "legal_actions should offer CastSpell for a flashback card in the graveyard");
}

/// A card with flashback in hand is an ordinary card: cast for {R}, resolved
/// into the graveyard.
///
/// The `CastSpell` action names only the object, so "is it offered?" cannot
/// tell a normal cast from a flashback cast. What separates them is what the
/// cast costs and where the card ends up (CR 702.33a): {R} rather than {3}{R},
/// and the graveyard rather than exile.
#[test]
fn a_flashback_card_in_hand_is_cast_normally() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = spell_in_hand(&mut state, &reg, "Geistflame", P0);
    // Exactly {R}: enough for the printed cost, one short of flashback's {3}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard,
        "cast from hand for {{R}}, so it goes to the graveyard — exile here would          mean the engine treated a hand cast as a flashback cast");
    assert_eq!(state.get_player(P1).life, 19, "and it resolved: Geistflame's 1 damage");
}

/// Flashback is NOT offered when the player lacks mana for the flashback cost.
#[test]
fn flashback_not_offered_without_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Geistflame").unwrap();
    let card = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(card).unwrap().name = "Geistflame".into();

    // Only {R} — not enough for flashback cost {3}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let has_flashback_cast = can_cast(&state, &reg, card);
    assert!(!has_flashback_cast,
        "Flashback should not be offered with insufficient mana");
}

/// A spell cast via flashback is exiled (not sent to graveyard) after resolution.
#[test]
fn flashback_spell_is_exiled_after_resolve() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Geistflame").unwrap();
    let card = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(card).unwrap().name = "Geistflame".into();

    // Flashback cost: {3}{R}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 4);

    state = cast_and_resolve(
        &state,
        &reg,
        card,
        vec![Target::Player(P1)],
    );

    assert_eq!(state.get_object(card).unwrap().zone, Zone::Exile,
        "Flashback spell should be exiled after resolution, not in graveyard");
}

/// A flashback spell that is countered should still be exiled (not graveyard).
#[test]
fn flashback_spell_countered_is_exiled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts Geistflame from graveyard via flashback.
    let gf_id = reg.get_id_by_name("Geistflame").unwrap();
    let gf = state.create_object(gf_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(gf).unwrap().name = "Geistflame".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 4);

    state = cast_onto_stack(&state, &reg, gf, vec![Target::Player(P1)]);

    // P1 casts Counterspell targeting Geistflame on the stack.
    let cs = spell_in_hand(&mut state, &reg, "Counterspell", P1);
    add_mana_for(&mut state, &reg, "Counterspell", P1);
    state.priority_player = Some(P1);

    state = cast_onto_stack(&state, &reg, cs, vec![Target::Object(gf)]);
    // Resolve the Counterspell (top of stack).
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(gf).unwrap().zone, Zone::Exile,
        "A flashback spell that is countered should be exiled");
    assert_eq!(state.get_object(cs).unwrap().zone, Zone::Graveyard,
        "Counterspell itself goes to graveyard normally");
}

/// `mill_cards` moves cards from library to graveyard.
#[test]
fn mill_cards_moves_to_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Manually stock P1's library with 5 cards.
    let lib_cards = stock_library(&mut state, &reg, P1, 5);

    // Mill 3 cards.
    engine::mill_cards(&mut state, P1, 3, "test", &reg);

    // First 3 should be in graveyard, last 2 remain in library.
    for &id in &lib_cards[0..3] {
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard,
            "Milled card should be in graveyard");
    }
    for &id in &lib_cards[3..5] {
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Library,
            "Non-milled card should remain in library");
    }
    assert_eq!(state.get_player(P1).library_order.len(), 2,
        "Library should have 2 cards remaining");
}

// ── Card-specific tests ────────────────────────────────────────────

/// Think Twice flashback: draw a card from graveyard, spell is exiled.
#[test]
fn think_twice_draws_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    stock_library(&mut state, &reg, P0, 1);

    let hand_before = state.objects_in_zone(Zone::Hand, P0).len();

    // Put Think Twice in graveyard. Flashback cost: {2}{U}.
    let tt_id = reg.get_id_by_name("Think Twice").unwrap();
    let tt = state.create_object(tt_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(tt).unwrap().name = "Think Twice".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 3);

    state = cast_and_resolve(&state, &reg, tt, vec![]);

    let hand_after = state.objects_in_zone(Zone::Hand, P0).len();
    assert_eq!(hand_after, hand_before + 1,
        "Think Twice should draw one card");
    assert_eq!(state.get_object(tt).unwrap().zone, Zone::Exile,
        "Think Twice cast via flashback should be exiled");
}

/// Dream Twist mills 3 cards from target player's library.
#[test]
fn dream_twist_mills_three() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Stock both libraries, so "whose" is a question the test can answer.
    let lib_cards = stock_library(&mut state, &reg, P1, 5);
    stock_library(&mut state, &reg, P0, 5);

    // Cast Dream Twist from hand. Cost: {U}.
    let dt = castable_spell(&mut state, &reg, "Dream Twist", P0);

    state = cast_and_resolve(&state, &reg, dt, vec![Target::Player(P1)]);

    // 3 of the 5 library cards should now be in graveyard.
    let gy_count = lib_cards.iter()
        .filter(|&&id| state.get_object(id).unwrap().zone == Zone::Graveyard)
        .count();
    assert_eq!(gy_count, 3, "Dream Twist should mill 3 cards");
    assert_eq!(state.get_player(P1).library_order.len(), 2,
        "P1 library should have 2 cards remaining");
    assert_eq!(state.get_player(P0).library_order.len(), 5,
        "and only the targeted player's — the caster's library is untouched");
}

/// "Target *player*", not "target opponent": the caster is a legal target and
/// milling yourself is a real reason to cast it here (Splinterfright, Spider
/// Spawning, and the flashback cards all want your own graveyard filled).
///
/// The pair of directions is what pins the target down. A resolve that always
/// milled the opponent, or always the caster, passes one of these two tests.
#[test]
fn dream_twist_mills_the_caster_when_it_targets_them() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = stock_library(&mut state, &reg, P0, 5);
    stock_library(&mut state, &reg, P1, 5);

    let dt = castable_spell(&mut state, &reg, "Dream Twist", P0);
    state = cast_and_resolve(&state, &reg, dt, vec![Target::Player(P0)]);

    let gy_count = mine.iter()
        .filter(|&&id| state.get_object(id).unwrap().zone == Zone::Graveyard)
        .count();
    assert_eq!(gy_count, 3, "the caster milled themselves");
    assert_eq!(state.get_player(P1).library_order.len(), 5,
        "and the opponent was not the target");
}

/// Travel Preparations adds a +1/+1 counter to a target creature.
#[test]
fn travel_preparations_adds_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast Travel Preparations from hand. Cost: {1}{G}.
    let tp = castable_spell(&mut state, &reg, "Travel Preparations", P0);

    state = cast_and_resolve(&state, &reg, tp, vec![Target::Object(creature)]);

    let counters = state.get_counter_count(creature, CounterType::PlusOnePlusOne);
    assert_eq!(counters, 1,
        "Travel Preparations should add a +1/+1 counter");
}

/// Ruling: "If Travel Preparations targets two creatures, and one of them is
/// an illegal target by the time Travel Preparations resolves, you'll still
/// put a +1/+1 counter on the other creature."
///
/// CR 608.2b: only a spell whose targets are *all* illegal is countered, and
/// the instruction skips the ones that are.
#[test]
fn travel_preparations_counters_the_creature_that_is_still_there() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let leaving = ready_creature(&mut state, P0, 2, 2);
    let staying = ready_creature(&mut state, P0, 2, 2);

    let tp = castable_spell(&mut state, &reg, "Travel Preparations", P0);
    let mut state = cast_onto_stack(&state, &reg, tp,
        vec![Target::Object(leaving), Target::Object(staying)]);

    state.move_object(leaving, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(counters_of(&state, staying, CounterType::PlusOnePlusOne), 1,
        "the surviving target still gets its counter");
}

/// Rolling Temblor deals 2 damage to each creature without flying.
/// Flyers are unaffected.
#[test]
fn rolling_temblor_damages_non_flyers() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let non_flyer = ready_creature(&mut state, P1, 3, 4);

    // Create a creature with flying.
    let flyer_card_id = reg.get_id_by_name("Chapel Geist").unwrap();
    let flyer = state.create_object(flyer_card_id, P1, Zone::Battlefield, Some(2), Some(3));
    state.get_object_mut(flyer).unwrap().name = "Chapel Geist".into();
    state.get_object_mut(flyer).unwrap().summoning_sick = false;
    state.get_object_mut(flyer).unwrap().keywords = vec![Keyword::Flying];

    // Cast Rolling Temblor. Cost: {2}{R}.
    let rt = castable_spell(&mut state, &reg, "Rolling Temblor", P0);

    state = cast_and_resolve(&state, &reg, rt, vec![]);

    assert_eq!(state.get_object(non_flyer).unwrap().damage_marked, 2,
        "Non-flyer should take 2 damage from Rolling Temblor");
    assert_eq!(state.get_object(flyer).unwrap().damage_marked, 0,
        "Flyer should take 0 damage from Rolling Temblor");
}

/// Unburial Rites returns a creature card from graveyard to battlefield.
#[test]
fn unburial_rites_returns_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a creature in P0's graveyard.
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();

    // Cast Unburial Rites. Cost: {4}{B}.
    let ur = castable_spell(&mut state, &reg, "Unburial Rites", P0);

    state = cast_and_resolve(&state, &reg, ur, vec![Target::Object(bears)]);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Battlefield,
        "Unburial Rites should return the creature to the battlefield");
    assert_eq!(state.get_object(bears).unwrap().controller, P0,
        "under the control of the player who put it there (CR 110.2a)");
    assert!(state.get_object(bears).unwrap().summoning_sick,
        "and as a new object (CR 400.7), so it is summoning sick — a \
         reanimated creature cannot attack the turn it arrives");
}

/// Gnaw to the Bone gains 2 life per creature card in the controller's graveyard.
#[test]
fn gnaw_to_the_bone_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 creature cards in P0's graveyard.
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..3 {
        let c = state.create_object(bears_id, P0, Zone::Graveyard, Some(2), Some(2));
        state.get_object_mut(c).unwrap().name = "Grizzly Bears".into();
    }

    let life_before = state.get_player(P0).life;

    // Cast Gnaw to the Bone. Cost: {2}{G}.
    let gnaw = castable_spell(&mut state, &reg, "Gnaw to the Bone", P0);

    state = cast_and_resolve(&state, &reg, gnaw, vec![]);

    assert_eq!(state.get_player(P0).life, life_before + 6,
        "Gnaw to the Bone should gain 2 life per creature in graveyard (3 creatures = 6 life)");
}

/// Desperate Ravings draws 2 cards, then discards 1. Net hand size change is +1.
#[test]
fn desperate_ravings_draws_two_discards_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Stock library with 3 cards to draw from.
    stock_library(&mut state, &reg, P0, 3);

    let hand_before = state.objects_in_zone(Zone::Hand, P0).len();

    // Cast Desperate Ravings. Cost: {1}{R}.
    let dr = castable_spell(&mut state, &reg, "Desperate Ravings", P0);

    state = cast_and_resolve(&state, &reg, dr, vec![]);

    let hand_after = state.objects_in_zone(Zone::Hand, P0).len();
    // `hand_before` is measured before Desperate Ravings is put in hand, so the
    // spell itself nets out: +2 drawn, -1 discarded.
    assert_eq!(hand_after, hand_before + 1,
        "Desperate Ravings should result in net +1 hand size (draw 2, discard 1, minus the spell)");
}

/// Forbidden Alchemy reveals top 4, player picks 1 for hand, rest go to graveyard.
#[test]
fn forbidden_alchemy_draws_and_mills() {
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Stock library with 5 cards.
    let lib_cards = stock_library(&mut state, &reg, P0, 5);

    let hand_before = state.objects_in_zone(Zone::Hand, P0).len();

    // Cast Forbidden Alchemy. Cost: {2}{U}.
    let fa = castable_spell(&mut state, &reg, "Forbidden Alchemy", P0);

    state = cast_and_resolve(&state, &reg, fa, vec![]);

    // Should now be awaiting a ChooseFromRevealed choice with 4 revealed cards.
    assert!(state.awaiting_action.is_some(), "Should be awaiting a choice");

    // Choose the first revealed card (lib_cards[0]) to put into hand.
    let keep_card = lib_cards[0];
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(keep_card) },
        &reg,
    );

    let hand_after = state.objects_in_zone(Zone::Hand, P0).len();
    // Cast from hand (-1), chose 1 to hand (+1) => net 0 from hand_before, but the chosen card is from library.
    assert_eq!(hand_after, hand_before + 1,
        "Forbidden Alchemy should put 1 card into hand (net +1 after spell leaves hand)");

    // 3 cards should have been sent to graveyard (the other revealed cards).
    let gy_lib_cards = lib_cards.iter()
        .filter(|&&id| state.get_object(id).unwrap().zone == Zone::Graveyard)
        .count();
    assert_eq!(gy_lib_cards, 3,
        "Forbidden Alchemy should put 3 revealed cards into graveyard");

    // Library should have 1 remaining (5 - 4 revealed; 1 untouched).
    assert_eq!(state.get_player(P0).library_order.len(), 1,
        "Library should have 1 card remaining after revealing 4 from 5");
}

/// Feeling of Dread taps a target creature.
#[test]
fn feeling_of_dread_taps_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    assert!(!state.get_object(creature).unwrap().tapped,
        "Creature should start untapped");

    // Cast Feeling of Dread. Cost: {1}{W}.
    let fod = castable_spell(&mut state, &reg, "Feeling of Dread", P0);

    state = cast_and_resolve(&state, &reg, fod, vec![Target::Object(creature)]);

    assert!(state.get_object(creature).unwrap().tapped,
        "Feeling of Dread should tap the target creature");
}

/// Ruling: "You can't target the same creature twice to put two +1/+1 counters
/// on it."
///
/// CR 601.2c — the same target can't be chosen twice for one instance of the
/// word "target", and "each of **up to two target creatures**" is one
/// instance. The engine's own action list already honours it (it enumerates
/// combinations), but both clients build their `CastSpell` from a per-slot
/// choice rather than picking a whole offered action, and neither checked:
/// an LLM answering `[0, 0]` doubled the counters.
#[test]
fn travel_preparations_cannot_target_the_same_creature_twice() {
    use mtg_engine::actions::Action;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let other = ready_creature(&mut state, P0, 2, 2);
    let prep = castable_spell(&mut state, &reg, "Travel Preparations", P0);

    let offers: Vec<Vec<Target>> = engine::legal_actions(&state, &reg).actions.into_iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if object_id == prep => Some(targets),
            _ => None,
        })
        .collect();
    assert!(offers.contains(&vec![Target::Object(creature), Target::Object(other)]),
        "test setup: two distinct creatures are offered together");
    assert!(!offers.iter().any(|t| t.len() == 2 && t[0] == t[1]),
        "no offered cast names the same creature twice. Offered: {offers:?}");

    // And the same list submitted by hand gets one counter, not two.
    let state = cast_and_resolve(&state, &reg, prep,
        vec![Target::Object(creature), Target::Object(creature)]);

    assert_eq!(counters_of(&state, creature, CounterType::PlusOnePlusOne), 1,
        "one instance of \"target\" means one counter, however the action was built");
}

/// A spell cast from a graveyard is not in that graveyard any more, so it can
/// never be one of its own targets.
///
/// CR 601.2a moves the card to the stack; CR 601.2c chooses targets after
/// that. Purify the Grave — "Exile target card from a graveyard", flashback
/// {W} — is the card that can ask, and it was offered a cast targeting itself.
/// Memory's Journey, flashback {G} with a graveyard-card slot, can ask the
/// same question.
#[test]
fn a_spell_cast_from_a_graveyard_is_not_offered_as_its_own_target() {
    use mtg_engine::actions::Action;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Purify the Grave").expect("in the registry");
    let purify = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(purify).unwrap().name = "Purify the Grave".into();
    // Something else to point at, so "no self-target offered" is not just
    // "nothing offered".
    let other = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    add_mana(&mut state, P0, &[(ManaType::White, 2)]);

    let offers: Vec<Vec<Target>> = engine::legal_actions(&state, &reg).actions.into_iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if object_id == purify => Some(targets),
            _ => None,
        })
        .collect();

    assert!(offers.contains(&vec![Target::Object(other)]),
        "the other graveyard card is offered");
    assert!(!offers.contains(&vec![Target::Object(purify)]),
        "but the spell itself is not: by the time targets are chosen it is on \
         the stack, not in a graveyard (CR 601.2a/c). Offered: {offers:?}");
}

/// Ruling: "If Feeling of Dread targets two creatures, and one of them is an
/// illegal target by the time Feeling of Dread resolves, the other creature
/// will still be tapped."
///
/// CR 608.2b: a spell is countered only when *every* target is illegal, and
/// the instructions skip the ones that are. One creature leaves in response,
/// the other taps.
#[test]
fn feeling_of_dread_taps_the_target_that_is_still_there() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let leaving = ready_creature(&mut state, P1, 3, 3);
    let staying = ready_creature(&mut state, P1, 3, 3);

    let fod = castable_spell(&mut state, &reg, "Feeling of Dread", P0);
    let mut state = cast_onto_stack(&state, &reg, fod,
        vec![Target::Object(leaving), Target::Object(staying)]);

    // In response, one of the two targets leaves the battlefield.
    state.move_object(leaving, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.get_object(staying).unwrap().tapped,
        "the target that is still a legal target is tapped");
    assert!(!state.get_object(leaving).unwrap().tapped,
        "and the one that left is not (leaving the battlefield untaps it in \
         any case — CR 400.7 makes it a new object)");
}

/// Bump in the Night flashback: opponent loses 3 life and Bump is exiled.
#[test]
fn bump_in_the_night_flashback_exiles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let life_before = state.get_player(P1).life;

    // Put Bump in the Night in graveyard. Flashback cost: {5}{R}.
    let bump_id = reg.get_id_by_name("Bump in the Night").unwrap();
    let bump = state.create_object(bump_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(bump).unwrap().name = "Bump in the Night".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 6);

    state = cast_and_resolve(&state, &reg, bump, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, life_before - 3,
        "Bump in the Night should cause opponent to lose 3 life");
    assert_eq!(state.get_object(bump).unwrap().zone, Zone::Exile,
        "Bump in the Night cast via flashback should be exiled");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// CR 702.33a: flashback is a way of casting the card, so anything that stops
/// the card being cast stops the flashback cast too. Nevermore's ban used to be
/// checked only on the cast-from-hand path.
#[test]
fn nevermore_stops_the_card_it_names_from_being_flashed_back() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Nevermore naming "Think Twice"
    let nevermore = named_permanent(&mut state, &registry, "Nevermore", P0);
    if let Some(obj) = state.get_object_mut(nevermore) {
        obj.instance_continuous_effects = Some(vec![
            ContinuousEffect::PreventCastingNamed { name: "Think Twice".into() },
        ]);
    }

    // Put Think Twice in P1's graveyard with flashback
    let think_twice = {
        let card_id = registry.get_id_by_name("Think Twice").unwrap();
        let id = state.create_object(card_id, P1, Zone::Graveyard, None, None);
        state.get_object_mut(id).unwrap().name = "Think Twice".into();
        id
    };

    // Add mana for flashback cost
    state.get_player_mut(P1).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P1).mana_pool.add(ManaType::Colorless, 2);
    state.priority_player = Some(P1);

    // Check legal actions for P1 — flashback Think Twice should NOT be available
    let legal = engine::legal_actions(&state, &registry);
    let can_flashback = legal.actions.iter().any(|a| {
        match a {
            Action::CastSpell { object_id, .. } => *object_id == think_twice,
            _ => false,
        }
    });

    assert!(!can_flashback,
        "Think Twice should not be castable via flashback while Nevermore names it");
}

/// "Each instant and sorcery card in your graveyard gains flashback until end
/// of turn. The flashback cost is equal to its mana cost" (CR 702.33a).
///
/// This used to cast Past in Flames with an empty graveyard and assert that no
/// card had been granted a free flashback — true of nothing at all, so it would
/// have passed however the grant was implemented. It now stocks the graveyard
/// and checks the grant that is actually made: the right cards, at the right
/// cost, castable for it.
#[test]
fn past_in_flames_grants_flashback_at_each_cards_own_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Think Twice is an instant costing {1}{U}; the Bears are a creature card,
    // which the ability does not reach.
    let think_twice = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    let bears = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);

    let pif = castable_spell(&mut state, &reg, "Past in Flames", P0);
    state = cast_and_resolve(&state, &reg, pif, vec![]);

    let granted: Vec<_> = state.until_end_of_turn.iter()
        .filter_map(|e| match e {
            mtg_engine::state::TemporaryEffect::GrantFlashback { target, cost } => Some((*target, cost)),
            _ => None,
        })
        .collect();

    let tt = granted.iter().find(|(t, _)| *t == think_twice)
        .unwrap_or_else(|| panic!("Think Twice should have gained flashback, granted: {granted:?}"));
    assert_eq!(tt.1.mana_value(), 2,
        "the flashback cost is equal to its mana cost, {{1}}{{U}} — an empty cost          here would make it castable for free");
    assert!(!granted.iter().any(|(t, _)| *t == bears),
        "a creature card in the graveyard gains nothing");

    // And the grant is real: with {1}{U} up, the engine offers the cast.
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    let legal = engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == think_twice)),
        "Think Twice should now be castable from the graveyard");
}

/// Ruling: "Past in Flames affects only cards in your graveyard at the time it
/// resolves. Instant and sorcery cards put into your graveyard later in the
/// turn won't gain flashback."
///
/// Brimstone Volley rather than Think Twice throughout the three tests below:
/// it is an instant with no flashback of its own, so the grant is the only
/// reason it could ever be cast from a graveyard.
#[test]
fn past_in_flames_does_not_reach_a_card_that_arrives_after_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pif = castable_spell(&mut state, &reg, "Past in Flames", P0);
    let mut state = cast_and_resolve(&state, &reg, pif, vec![]);

    // Only now does the Volley reach the graveyard.
    let volley = named_card_in_graveyard(&mut state, &reg, "Brimstone Volley", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 3);

    assert!(!state.until_end_of_turn.iter().any(|e|
        matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == volley)));
    let legal = engine::legal_actions(&state, &reg);
    assert!(!legal.actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == volley)),
        "it was not in the graveyard when the ability resolved");
}

/// "Each instant and sorcery card in **your** graveyard." An opponent's
/// graveyard is not yours (CR 404.3 — a card goes to its owner's).
#[test]
fn past_in_flames_leaves_an_opponents_graveyard_alone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let theirs = named_card_in_graveyard(&mut state, &reg, "Brimstone Volley", P1);
    let pif = castable_spell(&mut state, &reg, "Past in Flames", P0);
    let state = cast_and_resolve(&state, &reg, pif, vec![]);

    assert!(!state.until_end_of_turn.iter().any(|e|
        matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == theirs)),
        "the ability reaches one graveyard, and it is the caster's");
}

/// "gains flashback **until end of turn**" — so the next turn it is an
/// ordinary instant in a graveyard again.
#[test]
fn past_in_flames_flashback_grant_expires_at_end_of_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let volley = named_card_in_graveyard(&mut state, &reg, "Brimstone Volley", P0);
    let pif = castable_spell(&mut state, &reg, "Past in Flames", P0);
    let mut state = cast_and_resolve(&state, &reg, pif, vec![]);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 3);
    let legal = engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == volley)),
        "test setup: this turn it is castable from the graveyard");

    // Round the table back to the caster's main phase.
    advance_to_next_turn(&mut state, &reg);
    advance_to_next_turn(&mut state, &reg);
    advance_to_step(&mut state, &reg, Step::PrecombatMain);
    state.priority_player = Some(P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 3);

    assert!(state.until_end_of_turn.is_empty(), "cleanup took the grant with it");
    let legal = engine::legal_actions(&state, &reg);
    assert!(!legal.actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == volley)),
        "and it is an ordinary instant in a graveyard again");
}

/// Bug BS (`audits/AUDIT_BUGS.md)`: `cast_with_flashback` persisted on the
/// object when Runic Repetition returned an exiled flashback card to hand. The
/// next time that card was cast normally, `move_spell_after_resolve` saw the
/// stale flag and sent the card to exile instead of the graveyard.
///
/// Oracle (Runic Repetition): "Return target exiled card with
/// flashback you own to your hand."
///
/// Failure mode: `state.rs::move_object` cleared battlefield-related fields but
/// not `cast_with_flashback`, and the cast handler only ever SET the flag.
///
/// We put a Devil's Play in exile with `cast_with_flashback = true`,
/// move it back to hand via the engine's `move_object` (simulating
/// Runic Repetition), and assert the flag is now false.
#[test]
fn runic_repetition_clears_the_flashback_flag_on_the_returned_card() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils_card_id = registry.get_id_by_name("Devil's Play").unwrap();
    let devils = state.create_object(devils_card_id, P0, Zone::Exile, None, None);
    {
        let obj = state.get_object_mut(devils).unwrap();
        obj.name = "Devil's Play".into();
        obj.cast_with_flashback = true;
    }

    state.move_object(devils, Zone::Hand, &registry);

    let still_flashback = state
        .get_object(devils)
        .is_some_and(|o| o.cast_with_flashback);
    assert!(
        !still_flashback,
        "After Runic Repetition returns a flashback-cast card from \
         exile to hand, obj.cast_with_flashback should be reset. \
         Bug BS: move_object doesn't clear the flag, so the next \
         normal cast sends the card back to exile on resolution."
    );
}

// ─────────────────────────────────────────────────────────────────
// A *granted* flashback — Snapcaster Mage
// ─────────────────────────────────────────────────────────────────
//
// Everything above tests flashback printed on the card. A granted one takes a
// different route through the engine at every step: the cost comes from a
// `GrantFlashback` entry rather than `data.flashback_cost`, it lasts only
// until end of turn, and the card carries no flashback of its own to fall back
// on. Each of these is the granted twin of a printed-flashback test above.

/// Put Mulch — {1}{G} sorcery, no flashback of its own — in P0's graveyard and
/// hand its Snapcaster grant back, through the real ETB trigger.
fn mulch_with_a_granted_flashback(
    reg: &CardRegistry,
    state: &mut mtg_engine::state::GameState,
) -> ObjectId {
    let mulch = named_card_in_graveyard(state, reg, "Mulch", P0);
    let snap = castable_spell(state, reg, "Snapcaster Mage", P0);
    let mut next = cast_onto_stack(state, reg, snap, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut next, reg);
    mtg_engine::triggers::process_triggers(&mut next, reg);
    *state = next;
    mulch
}

/// "...gains flashback **until end of turn**." That is the only durational
/// clause on the card, and a grant that outlived the turn would look correct
/// in every other test here.
#[test]
fn a_granted_flashback_is_gone_next_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // Two turn cycles of draw steps ahead: without libraries both players deck
    // out, the game ends, and *every* action disappears — which would make
    // this test pass against a grant that never expired.
    stock_library(&mut state, &reg, P0, 10);
    stock_library(&mut state, &reg, P1, 10);
    let mulch = mulch_with_a_granted_flashback(&reg, &mut state);

    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);
    assert!(can_cast(&state, &reg, mulch),
        "test precondition: the grant is live on the turn it was made");

    // Two turns, back round to P0's own main phase. Stopping after one would
    // land in P1's turn, where P0 could not cast a sorcery whatever the grant
    // said — and the test would pass against a grant that never expired.
    advance_to_next_turn(&mut state, &reg);
    advance_to_next_turn(&mut state, &reg);
    advance_to_step(&mut state, &reg, Step::PrecombatMain);
    assert_eq!(state.active_player, P0, "test precondition: back in P0's turn");
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);
    assert!(!mtg_engine::engine::legal_actions(&state, &reg).actions.is_empty(),
        "test precondition: the game is still running, so 'cannot cast' below \
         means the grant expired and not that there is nothing to do");

    assert!(!can_cast(&state, &reg, mulch),
        "the grant lasted until end of turn, so Mulch is an ordinary card in \
         the graveyard again");
}

/// Ruling: "You must still follow any timing restrictions and permissions,
/// including those based on the card's type. For instance, you can cast a
/// sorcery using flashback only when you could normally cast a sorcery."
///
/// This is the trap the card is famous for: Snapcaster has flash, so it can
/// enter on an opponent's turn, and a sorcery it grants flashback to cannot be
/// cast before the grant expires.
#[test]
fn a_granted_flashback_on_a_sorcery_still_obeys_sorcery_timing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mulch = mulch_with_a_granted_flashback(&reg, &mut state);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    assert!(can_cast(&state, &reg, mulch),
        "test precondition: castable in P0's own main phase");

    // Same grant, same mana, a step where no sorcery may be cast.
    advance_to_step(&mut state, &reg, Step::EndStep);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    assert!(!can_cast(&state, &reg, mulch),
        "a sorcery cast via flashback still needs sorcery timing — the grant \
         is not a permission to cast it whenever");
}

/// Ruling: "A spell cast using flashback will always be exiled afterward,
/// whether it resolves, is countered, or leaves the stack in some other way."
/// Tested above for a printed flashback cost; the granted path sets the same
/// flag from a different branch, so it needs its own case.
#[test]
fn a_spell_cast_with_a_granted_flashback_is_exiled_after_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mulch = mulch_with_a_granted_flashback(&reg, &mut state);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    let cast = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == mulch))
        .expect("Mulch is castable from the graveyard on its granted flashback");
    let mut state = mtg_engine::engine::submit_action(&state, &cast, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(mulch).unwrap().zone, Zone::Exile,
        "cast via flashback, so it is exiled rather than returning to the \
         graveyard — and the flag is set on the granted branch too");
}

/// "target instant or sorcery card in **your** graveyard." CR 404.3 puts a
/// card in its owner's graveyard, so an opponent's is out of reach.
#[test]
fn snapcaster_cannot_reach_an_instant_in_an_opponents_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let theirs = named_card_in_graveyard(&mut state, &reg, "Think Twice", P1);
    let snap = castable_spell(&mut state, &reg, "Snapcaster Mage", P0);
    let mut state = cast_onto_stack(&state, &reg, snap, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert!(!state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == theirs)),
        "the only instant in the game is in the opponent's graveyard, which is \
         not \"your graveyard\" — so the trigger had no legal target at all");
}

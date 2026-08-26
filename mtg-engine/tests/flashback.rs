//! Tests for the flashback mechanic and all 15 flashback cards.
//!
//! Flashback allows casting a spell from the graveyard for an alternative cost.
//! After resolution (or countering), the spell is exiled instead of returning to graveyard.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

// ── System tests: flashback mechanics ──────────────────────────────

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

    let legal = engine::legal_actions(&state, &reg);
    let has_flashback_cast = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == card)
    });
    assert!(has_flashback_cast,
        "legal_actions should offer CastSpell for a flashback card in the graveyard");
}

/// A card with flashback in the hand should only be castable at its normal cost,
/// not via flashback.
#[test]
fn flashback_not_offered_from_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = spell_in_hand(&mut state, &reg, "Geistflame", P0);

    // Give exactly {R} (enough for normal Geistflame, not enough for {3}{R} flashback).
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let has_cast = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == card)
    });
    // It IS offered — as a normal cast from hand at cost {R}.
    assert!(has_cast,
        "Geistflame should be castable from hand at normal cost R");
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

    let legal = engine::legal_actions(&state, &reg);
    let has_flashback_cast = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == card)
    });
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

/// A spell cast normally from hand goes to graveyard after resolution.
#[test]
fn normal_cast_goes_to_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = castable_spell(&mut state, &reg, "Geistflame", P0);

    state = cast_and_resolve(
        &state,
        &reg,
        card,
        vec![Target::Player(P1)],
    );

    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard,
        "Normal cast should go to graveyard after resolution");
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

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: gf, targets: vec![Target::Player(P1)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // P1 casts Counterspell targeting Geistflame on the stack.
    let cs = spell_in_hand(&mut state, &reg, "Counterspell", P1);
    add_mana_for(&mut state, &reg, "Counterspell", P1);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: cs, targets: vec![Target::Object(gf)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
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
    let mountain_id = reg.get_id_by_name("Mountain").unwrap();
    let mut lib_cards = Vec::new();
    for _ in 0..5 {
        let obj = state.create_object(mountain_id, P1, Zone::Library, None, None);
        state.get_object_mut(obj).unwrap().name = "Mountain".into();
        lib_cards.push(obj);
    }
    state.get_player_mut(P1).library_order = lib_cards.clone();

    // Mill 3 cards.
    engine::mill_cards(&mut state, P1, 3, &reg);

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

    // Stock library so there's a card to draw.
    let mountain_id = reg.get_id_by_name("Mountain").unwrap();
    let lib_card = state.create_object(mountain_id, P0, Zone::Library, None, None);
    state.get_object_mut(lib_card).unwrap().name = "Mountain".into();
    state.get_player_mut(P0).library_order = vec![lib_card];

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

    // Stock P1's library with 5 cards.
    let mountain_id = reg.get_id_by_name("Mountain").unwrap();
    let mut lib_cards = Vec::new();
    for _ in 0..5 {
        let obj = state.create_object(mountain_id, P1, Zone::Library, None, None);
        state.get_object_mut(obj).unwrap().name = "Mountain".into();
        lib_cards.push(obj);
    }
    state.get_player_mut(P1).library_order = lib_cards.clone();

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
    let mountain_id = reg.get_id_by_name("Mountain").unwrap();
    let mut lib_cards = Vec::new();
    for _ in 0..3 {
        let obj = state.create_object(mountain_id, P0, Zone::Library, None, None);
        state.get_object_mut(obj).unwrap().name = "Mountain".into();
        lib_cards.push(obj);
    }
    state.get_player_mut(P0).library_order = lib_cards;

    let hand_before = state.objects_in_zone(Zone::Hand, P0).len();

    // Cast Desperate Ravings. Cost: {1}{R}.
    let dr = castable_spell(&mut state, &reg, "Desperate Ravings", P0);

    state = cast_and_resolve(&state, &reg, dr, vec![]);

    let hand_after = state.objects_in_zone(Zone::Hand, P0).len();
    // Before: hand_before cards + Desperate Ravings (which goes to stack then graveyard).
    // The spell is cast from hand (removing it), then draws 2, discards 1 => net +1 from the
    // state before casting. But hand_before was measured before we created the spell card.
    // So: hand_before (0) -> +1 (create DR) -> -1 (cast DR) -> +2 (draw) -> -1 (discard) = 1
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
    let mountain_id = reg.get_id_by_name("Mountain").unwrap();
    let mut lib_cards = Vec::new();
    for _ in 0..5 {
        let obj = state.create_object(mountain_id, P0, Zone::Library, None, None);
        state.get_object_mut(obj).unwrap().name = "Mountain".into();
        lib_cards.push(obj);
    }
    state.get_player_mut(P0).library_order = lib_cards.clone();

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

/// Nightbird's Clutches prevents target creatures from blocking this turn.
#[test]
fn nightbirds_clutches_taps_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    assert!(!state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::CantBlock { target } if *target == creature)),
        "Creature should start able to block");

    // Cast Nightbird's Clutches. Cost: {1}{R}.
    let nc = castable_spell(&mut state, &reg, "Nightbird's Clutches", P0);

    state = cast_and_resolve(&state, &reg, nc, vec![Target::Object(creature)]);

    assert!(state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::CantBlock { target } if *target == creature)),
        "Nightbird's Clutches should prevent the target creature from blocking");
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

/// Bug: Nevermore bans a card by name, but the ban isn't checked
/// when casting that card via flashback from the graveyard.
#[test]
fn bug_nevermore_not_enforced_for_flashback() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Nevermore naming "Think Twice"
    let nevermore = named_creature(&mut state, &registry, "Nevermore", P0);
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

    // BUG: Nevermore ban doesn't apply to flashback casts
    assert!(!can_flashback,
        "Think Twice should not be castable via flashback while Nevermore names it");
}

/// Bug: Past in Flames gives flashback equal to a card's mana cost,
/// but cards with no mana cost get `ManaCost::free()`, making them
/// castable for free from the graveyard.
#[test]
fn bug_past_in_flames_free_flashback_for_no_cost_cards() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cast Past in Flames
    let pif = castable_spell(&mut state, &registry, "Past in Flames", P0);
    state = cast_and_resolve(&state, &registry, pif, vec![]);

    // Check the until_end_of_turn flashback entries
    // Any card with cost=None should NOT get flashback (or should get cost=None flashback
    // which is uncastable), not ManaCost::free()
    let free_flashbacks: Vec<_> = state.until_end_of_turn.iter()
        .filter_map(|e| if let mtg_engine::state::TemporaryEffect::GrantFlashback { cost, .. } = e {
            Some(cost)
        } else { None })
        .filter(|cost| cost.symbols.is_empty())
        .collect();

    // BUG: Cards with no mana cost get ManaCost::free() flashback
    assert!(free_flashbacks.is_empty(),
        "Cards with no mana cost should not get free flashback. Found {} free flashback entries",
        free_flashbacks.len());
}

/// Bug BS (`audits/AUDIT_BUGS.md)`: `cast_with_flashback` persists on
/// the object when Runic Repetition returns an exiled flashback
/// card to hand. The next time that card is cast normally,
/// `move_spell_after_resolve` sees the stale flag and sends the
/// card to exile instead of graveyard.
///
/// Oracle (Runic Repetition): "Return target exiled card with
/// flashback you own to your hand."
///
/// Failure mode: `state.rs::move_object` clears battlefield-related
/// fields but does not reset `cast_with_flashback`. The cast handler
/// only SETS the flag when `is_flashback = true`; it never clears it
/// on a normal cast.
///
/// We put a Devil's Play in exile with `cast_with_flashback = true`,
/// move it back to hand via the engine's `move_object` (simulating
/// Runic Repetition), and assert the flag is now false.
#[test]
fn bug_bs_runic_repetition_resets_cast_with_flashback() {
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

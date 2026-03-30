//! Tests for card fixes identified in the audit.
//! These tests document CORRECT behavior per Oracle text.
//! They should FAIL against buggy implementations and PASS after fixes.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ════════════════════════════════════════════════════════════════════
// Fiend Hunter (#2): "you may exile another target creature"
//
// - Should be optional ("you may")
// - Player should choose the target (not auto-pick strongest)
// - Can target ANY creature (including own), not just opponent's
// ════════════════════════════════════════════════════════════════════

/// Fiend Hunter should be able to target own creatures, not just opponent's.
#[test]
fn fiend_hunter_can_target_own_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has a creature they might want to "save" by exiling temporarily.
    let own_creature = named_creature(&mut state, &reg, "Grizzly Bears", P0);

    // Cast and resolve Fiend Hunter for P0.
    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::process_triggers(&mut state, &reg);

    // The hunter should have the ability to target own_creature.
    // Currently it only targets opponent creatures, so own_creature
    // should NOT have been exiled (bug: it auto-exiles opponent's strongest).
    // After fix: a ResolutionChoice should be presented with own_creature
    // as an option.
    //
    // For now, check that if only own creatures exist, the hunter still
    // has a valid target (doesn't just do nothing).
    let own_creature_zone = state.get_object(own_creature).unwrap().zone;

    // Fiend Hunter should present an optional choice including own creature.
    assert!(state.awaiting_action.is_some(),
        "Fiend Hunter should present a choice (not auto-target). \
         Own creature zone: {:?}", own_creature_zone);
}

/// Fiend Hunter should present a choice when multiple targets exist.
#[test]
fn fiend_hunter_presents_choice_with_multiple_targets() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Both players have creatures.
    let _p0_creature = named_creature(&mut state, &reg, "Grizzly Bears", P0);
    let _p1_creature_a = ready_creature(&mut state, P1, 3, 3);
    let _p1_creature_b = ready_creature(&mut state, P1, 2, 2);

    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::process_triggers(&mut state, &reg);

    // With 3 potential targets (own Bears + two opponent creatures),
    // a choice should be presented, not auto-targeted.
    assert!(state.awaiting_action.is_some(),
        "Fiend Hunter should present a target choice with multiple valid targets");
}

// ════════════════════════════════════════════════════════════════════
// Ranger's Guile (#3): "target creature you control"
// ════════════════════════════════════════════════════════════════════

/// Ranger's Guile should NOT be castable targeting an opponent's creature.
#[test]
fn rangers_guile_cannot_target_opponent_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature. P0 has Ranger's Guile.
    let _enemy = ready_creature(&mut state, P1, 3, 3);
    let _rg = castable_spell(&mut state, &reg, "Ranger's Guile", P0);

    let legal = engine::legal_actions(&state, &reg);

    // Ranger's Guile should NOT appear with an opponent's creature as target.
    let targets_enemy = legal.actions.iter().any(|a| {
        if let Action::CastSpell { targets, .. } = a {
            targets.iter().any(|t| {
                if let Target::Object(id) = t { *id == _enemy } else { false }
            })
        } else {
            false
        }
    });
    assert!(!targets_enemy,
        "Ranger's Guile should not be able to target opponent's creature (Oracle: 'you control')");
}

// ════════════════════════════════════════════════════════════════════
// Morkrut Banshee (#4): can target itself
// ════════════════════════════════════════════════════════════════════

/// Morkrut Banshee should be able to target itself with -4/-4.
#[test]
fn morkrut_banshee_can_target_self() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true; // enable morbid

    let banshee = castable_spell(&mut state, &reg, "Morkrut Banshee", P0);
    let mut state = cast_and_resolve(&state, &reg, banshee, vec![]);
    triggers::process_triggers(&mut state, &reg);

    // With only Morkrut Banshee on the battlefield, it should be able
    // to target itself (the only valid target). Currently it excludes
    // self and does nothing.
    let banshee_obj = state.objects.values()
        .find(|o| o.name == "Morkrut Banshee" && o.zone == Zone::Battlefield);

    if let Some(b) = banshee_obj {
        // If the banshee targeted itself with -4/-4, effective toughness
        // should be 0 (4 base - 4 = 0).
        let eff_t = state.effective_toughness(b.id, &reg);
        assert_eq!(eff_t, Some(0),
            "Morkrut Banshee should target itself when it's the only creature (morbid). \
             Effective toughness should be 0 (4 - 4). Got: {:?}", eff_t);
    } else {
        // Banshee might have already died from the -4/-4 SBA. That's also fine.
        // (It would be in graveyard.)
        let in_gy = state.objects.values()
            .any(|o| o.name == "Morkrut Banshee" && o.zone == Zone::Graveyard);
        assert!(in_gy,
            "Morkrut Banshee should have targeted itself and died from -4/-4");
    }
}

// ════════════════════════════════════════════════════════════════════
// Frightful Delusion (#6): discard always happens
// ════════════════════════════════════════════════════════════════════

/// Frightful Delusion: opponent should discard even when they pay {1}.
#[test]
fn frightful_delusion_discard_on_pay() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 casts a creature.
    let creature = castable_spell(&mut state, &reg, "Grizzly Bears", P1);
    state.priority_player = Some(P1);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: creature, targets: vec![] },
        &reg,
    );

    // P0 casts Frightful Delusion targeting the creature spell.
    state.priority_player = Some(P0);
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fd, targets: vec![Target::Object(creature)] },
        &reg,
    );

    // Give P1 mana to pay {1} and a card in hand to discard.
    state.get_player_mut(P1).mana_pool.add(ManaType::Colorless, 1);
    let discard_card = state.create_object(CardId(9999), P1, Zone::Hand, None, None);

    // Resolve Frightful Delusion. P1 should get a pay-or-not choice.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // P1 pays {1} to keep their spell.
    if let Some(mtg_engine::state::AwaitingAction::ResolutionChoice { .. }) = &state.awaiting_action {
        state = engine::submit_action(
            &state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::PayDecision(true),
            },
            &reg,
        );
    }

    // After paying, P1 should STILL have to discard a card.
    // Oracle: "Counter target spell unless its controller pays {1}. That player discards a card."
    // The discard is a separate effect that always happens.
    let hand_count = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_count, 0,
        "Frightful Delusion: opponent should discard even after paying mana. \
         Hand has {} cards (should be 0)", hand_count);
}

// ════════════════════════════════════════════════════════════════════
// Murder of Crows (#5): optional draw + player-chosen discard
// ════════════════════════════════════════════════════════════════════

/// Murder of Crows: when another creature dies, the controller should get
/// a choice to draw (optional). If they draw, they must choose a card to discard.
#[test]
fn murder_of_crows_presents_draw_choice() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has Murder of Crows.
    let _crows = named_creature(&mut state, &reg, "Murder of Crows", P0);

    // Give P0 some cards in hand so the discard has options.
    let hand_card = state.create_object(CardId(9999), P0, Zone::Hand, None, None);
    state.get_object_mut(hand_card).unwrap().name = "Hand Card".into();

    // A creature dies (P1's).
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // Give P0 library cards to draw from.
    let lib_card = state.create_object(CardId(9999), P0, Zone::Library, None, None);
    state.get_object_mut(lib_card).unwrap().name = "Library Card".into();
    state.get_player_mut(P0).library_order.push(lib_card);

    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    triggers::process_triggers(&mut state, &reg);

    // After the creature dies and trigger processes, Murder of Crows should
    // present a "you may draw" choice (or auto-draw and present discard choice).
    // Either way, the player should have agency — not auto-draw and auto-discard.
    //
    // Check: the player should end up with a choice to make.
    // The hand should have gained a card (the draw) and the player should
    // be asked to discard.
    let awaiting = state.awaiting_action.is_some();
    let hand_count = state.objects_in_zone(Zone::Hand, P0).len();

    // After fix: Murder of Crows draws a card, then presents a discard
    // choice since the player now has 2+ cards in hand.
    assert!(awaiting,
        "Murder of Crows should present a discard choice after drawing. \
         Hand: {}", hand_count);
    // Hand should have the original card + the drawn card = 2 cards.
    assert_eq!(hand_count, 2,
        "Should have 2 cards in hand (original + drawn) before discarding");
}

// ════════════════════════════════════════════════════════════════════
// Bramblecrush (#7): destruction pipeline for non-creatures
// (Already fixed, but verify with test)
// ════════════════════════════════════════════════════════════════════

/// Bramblecrush should use the destruction pipeline for non-creature permanents.
/// An indestructible enchantment should survive Bramblecrush.
#[test]
fn bramblecrush_respects_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a non-creature permanent (enchantment) with indestructible.
    let enchantment = state.create_object(CardId(9999), P1, Zone::Battlefield, None, None);
    state.get_object_mut(enchantment).unwrap().name = "Indestructible Enchantment".into();
    state.get_object_mut(enchantment).unwrap().card_types = vec![CardType::Enchantment];
    state.until_end_of_turn_keywords.push(
        mtg_engine::state::UntilEndOfTurnKeyword {
            target: enchantment,
            keyword: Keyword::Indestructible,
        },
    );

    let crush = castable_spell(&mut state, &reg, "Bramblecrush", P0);
    state = cast_and_resolve(&state, &reg, crush, vec![Target::Object(enchantment)]);

    // Indestructible enchantment should survive.
    assert_eq!(state.get_object(enchantment).unwrap().zone, Zone::Battlefield,
        "Bramblecrush should respect indestructible on non-creature permanents");
}

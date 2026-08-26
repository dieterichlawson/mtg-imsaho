//! Regressions found by auditing individual card implementations.
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Bramblecrush
//! - Fiend Hunter
//! - Frightful Delusion
//! - Morkrut Banshee
//! - Murder of Crows
//! - Ranger's Guile

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ════════════════════════════════════════════════════════════════════
// Fiend Hunter (#2): "you may exile another target creature"
//
// - Should be optional ("you may")
// - Player should choose the target (not auto-pick strongest)
// - Can target ANY creature (including own), not just opponent's
// ════════════════════════════════════════════════════════════════════

/// "When Fiend Hunter enters the battlefield, you may exile another target
/// creature." Another — not another *opponent's*: your own creature is a legal
/// choice, and a real one (exiling it dodges a sweeper, and it comes back when
/// the Hunter leaves).
///
/// Both of the tests this replaces asserted only that *a* choice was presented,
/// which says nothing about who is in it — an engine auto-exiling the
/// opponent's biggest creature and then asking an unrelated question passes
/// that.
#[test]
fn fiend_hunter_offers_every_creature_but_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let own = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = ready_creature(&mut state, P1, 3, 3);
    let theirs_too = ready_creature(&mut state, P1, 2, 2);

    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let options = pending_choice_options(&state);
    for (id, who) in [(own, "your own creature"), (theirs, "an opponent's"), (theirs_too, "and the other")] {
        assert!(options.contains(&Target::Object(id)),
            "{who} is a legal choice for 'another target creature'; offered {options:?}");
    }
    assert!(!options.contains(&Target::Object(hunter)),
        "'another' excludes the Fiend Hunter itself");

    // Nothing has been exiled while the choice is still pending.
    for id in [own, theirs, theirs_too] {
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    }
}

// ════════════════════════════════════════════════════════════════════
// Ranger's Guile (#3): "target creature you control"
// ════════════════════════════════════════════════════════════════════

/// "Target creature you control gets +1/+1 and gains hexproof until end of
/// turn." You control — so an opponent's creature is not offered, and one of
/// yours is. The second half matters: without it, an engine offering no
/// targets at all satisfies the first.
#[test]
fn rangers_guile_targets_only_your_own_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 2, 2);
    let theirs = ready_creature(&mut state, P1, 3, 3);
    let guile = castable_spell(&mut state, &reg, "Ranger's Guile", P0);

    let offered = offered_targets(&state, &reg, guile);
    assert!(offered.contains(&Target::Object(mine)), "your own creature; offered {offered:?}");
    assert!(!offered.contains(&Target::Object(theirs)), "not the opponent's");
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

    // With only Morkrut Banshee on the battlefield, it is the only legal
    // target for its own morbid ETB, so it must target itself. Either it is
    // still there at 4-4=0 toughness on its way to dying, or SBA has already
    // taken it — both mean the -4/-4 landed; nothing on the battlefield at
    // full toughness would.
    let obj = state.objects.values()
        .find(|o| o.name == "Morkrut Banshee")
        .expect("the Banshee is somewhere");
    match obj.zone {
        // Still on the battlefield, on its way out: 4 base toughness less 4.
        Zone::Battlefield => assert_eq!(state.effective_toughness(obj.id, &reg), Some(0),
            "the -4/-4 landed on the only legal target, which is itself"),
        // Or state-based actions already took it, which means the same thing.
        Zone::Graveyard => {}
        other => panic!("the Banshee should be on the battlefield at 0 toughness or \
                         already in the graveyard, not in {other:?}"),
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
    state = cast_onto_stack(&state, &reg, creature, vec![]);

    // P0 casts Frightful Delusion targeting the creature spell.
    state.priority_player = Some(P0);
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P0);
    state = cast_onto_stack(&state, &reg, fd, vec![Target::Object(creature)]);

    // Give P1 mana to pay {1} and a card in hand to discard.
    state.get_player_mut(P1).mana_pool.add(ManaType::Colorless, 1);
    let _discard_card = state.create_object(CardId(9999), P1, Zone::Hand, None, None);

    // Resolve Frightful Delusion. P1 should get a pay-or-not choice.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // P1 pays {1} to keep their spell. Asserted rather than tested for: with the
    // payment inside an `if`, a Frightful Delusion that stopped asking would
    // never pay and the discard below would be measuring the wrong scenario.
    assert!(state.awaiting_action.is_some(),
        "CR 608.2g: the spell's controller is asked whether to pay {{1}}");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::PayDecision(true),
        },
        &reg,
    );

    // After paying, P1 should STILL have to discard a card.
    // Oracle: "Counter target spell unless its controller pays {1}. That player discards a card."
    // The discard is a separate effect that always happens.
    let hand_count = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_count, 0,
        "Frightful Delusion: opponent should discard even after paying mana. \
         Hand has {hand_count} cards (should be 0)");
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
    let _crows = named_permanent(&mut state, &reg, "Murder of Crows", P0);

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
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    // Murder of Crows should present a "you may draw" yes/no choice.
    assert!(state.awaiting_action.is_some(),
        "Murder of Crows should present a yes/no draw choice");
    // Hand should still have 1 card (draw hasn't happened yet — waiting for choice).
    let hand_count = state.objects_in_zone(Zone::Hand, P0).len();
    assert_eq!(hand_count, 1,
        "Draw should NOT have happened yet (waiting for 'you may' choice)");
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
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword {
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

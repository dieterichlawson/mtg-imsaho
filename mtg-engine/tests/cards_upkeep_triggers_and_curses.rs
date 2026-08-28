//! Cards that trigger at the beginning of an upkeep, including the Curses whose
//! upkeep is the enchanted player's rather than their controller's (CR 603.2).
//!
//! Cards covered (11), so this is greppable by name as well as by rule:
//!
//! - Angel of Flight Alabaster
//! - Bloodgift Demon
//! - Boneyard Wurm
//! - Charmbreaker Devils
//! - Curse of Death's Hold
//! - Curse of Oblivion
//! - Curse of the Bloody Tome
//! - Curse of the Nightly Hunt
//! - Curse of the Pierced Heart
//! - Endless Ranks of the Dead
//! - Splinterfright
//!
//! Reaper from the Abyss's morbid end-step trigger is in `intervening_if.rs`,
//! with the rest of CR 603.4.

mod common;

use common::*;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ── Boneyard Wurm ─────────────────────────────────────────────────

/// Boneyard Wurm P/T = creature cards in graveyard.
#[test]
fn boneyard_wurm_pt_equals_creatures_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wurm = named_permanent(&mut state, &reg, "Boneyard Wurm", P0);

    // No creatures in graveyard yet.
    assert_eq!(state.effective_power(wurm, &reg).unwrap(), 0);
    assert_eq!(state.effective_toughness(wurm, &reg).unwrap(), 0);

    // Put 3 creatures in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    assert_eq!(state.effective_power(wurm, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(wurm, &reg).unwrap(), 3);
}

// ── Splinterfright ────────────────────────────────────────────────

/// Splinterfright mills 2 on upkeep.
#[test]
fn splinterfright_mills_on_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _splinter = named_permanent(&mut state, &reg, "Splinterfright", P0);

    stock_library(&mut state, &reg, P0, 4);

    // Fire upkeep trigger.
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Should have milled 2.
    let gy_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0)
        .count();
    assert_eq!(gy_count, 2, "Splinterfright should mill 2 cards on upkeep");
}

// ── Bloodgift Demon ───────────────────────────────────────────────

/// CR 113.7a: "target player draws a card and loses 1 life" is entirely about
/// the target — the Demon is not mentioned — so killing the Demon in response
/// to its own upkeep trigger does not stop the draw. The handler used to return
/// early once the source had left the battlefield.
#[test]
fn bloodgift_demons_trigger_resolves_even_if_the_demon_dies_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let demon = named_permanent(&mut state, &reg, "Bloodgift Demon", P0);
    stock_library(&mut state, &reg, P0, 1);

    // The trigger goes on the stack and its target is chosen (CR 603.3b).
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    triggers::collect_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "the target is chosen on the way to the stack");
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(
                Some(mtg_engine::actions::Target::Player(P0))),
        },
        &reg,
    );

    // The Demon is killed in response.
    state.move_object(demon, Zone::Graveyard, &reg);

    triggers::resolve_next_trigger(&mut state, &reg);

    let hand = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand, 1, "the card is drawn even though the Demon is gone");
    assert_eq!(state.get_player(P0).life, 19, "and the life is still lost");
}

/// Bloodgift Demon draws a card and loses 1 life on upkeep.
#[test]
fn bloodgift_demon_draws_and_loses_life() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _demon = named_permanent(&mut state, &reg, "Bloodgift Demon", P0);

    stock_library(&mut state, &reg, P0, 1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // CR 603.3b: the target is chosen as the trigger goes on the stack, so the
    // prompt comes first and the trigger resolves afterwards. (It used to be
    // the other way round — the trigger resolved and then asked, which is the
    // bug bloodgift_demon-01 reported.)
    assert!(state.awaiting_action.is_some(), "Should be awaiting player choice");
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(mtg_engine::actions::Target::Player(P0))),
        },
        &reg,
    );
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    let hand = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand, 1, "Should have drawn 1 card");
    assert_eq!(state.get_player(P0).life, 19, "Should have lost 1 life");
}

// ── Endless Ranks of the Dead ─────────────────────────────────────

/// Creates Zombie tokens equal to half your Zombies.
#[test]
fn endless_ranks_creates_zombie_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _enchantment = named_permanent(&mut state, &reg, "Endless Ranks of the Dead", P0);

    // Create 5 Zombies on the battlefield.
    for _ in 0..5 {
        let z = state.create_token_with_subtypes(
            "Zombie", P0, 2, 2, vec![Color::Black],
            vec![CardType::Creature], vec![], vec!["Zombie".into()],
            &reg,
        )[0];
        state.get_object_mut(z).unwrap().summoning_sick = false;
    }

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // 5 / 2 = 2 (rounded down). So 5 original + 2 new = 7 Zombies.
    let zombie_count = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.subtypes.iter().any(|s| s == "Zombie"))
        .count();
    assert_eq!(zombie_count, 7, "Should create 2 Zombie tokens (5/2 = 2)");
}

// ── Curse of the Pierced Heart ────────────────────────────────────

/// Curse deals 1 damage to enchanted player on their upkeep.
#[test]
fn curse_of_pierced_heart_deals_damage_on_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1); // P1's upkeep

    // P0 controls the curse attached to P1.
    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_player(P1).life, 19, "Curse should deal 1 damage to P1");
    assert_eq!(state.get_player(P0).life, 20, "P0 should be unaffected");
}

// ── Curse of Death's Hold ─────────────────────────────────────────

/// Curse gives opponent's creatures -1/-1.
#[test]
fn curse_of_deaths_hold_debuffs_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 controls curse targeting P1.
    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Death's Hold", P0, P1);

    // P1's creature.
    let opp_creature = ready_creature(&mut state, P1, 3, 3);
    // P0's creature (should NOT be affected).
    let own_creature = ready_creature(&mut state, P0, 3, 3);

    let opp_power = state.effective_power(opp_creature, &reg).unwrap();
    let opp_toughness = state.effective_toughness(opp_creature, &reg).unwrap();
    let own_power = state.effective_power(own_creature, &reg).unwrap();

    assert_eq!(opp_power, 2, "Opponent's creature should have -1 power");
    assert_eq!(opp_toughness, 2, "Opponent's creature should have -1 toughness");
    assert_eq!(own_power, 3, "Own creature should be unaffected");
}

// ── Angel of Flight Alabaster ─────────────────────────────────────

/// Angel returns a Spirit from graveyard on upkeep (single target auto-applies).
#[test]
fn angel_of_flight_alabaster_returns_spirit() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _angel = named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);

    // Put a Spirit in graveyard.
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Single Spirit → auto-applied (mandatory with 1 target).
    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "Spirit should be returned to hand");
}

// ── Charmbreaker Devils ───────────────────────────────────────────

/// Charmbreaker Devils gets +4/+0 when you cast an instant or sorcery.
#[test]
fn charmbreaker_devils_plus4_on_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_permanent(&mut state, &reg, "Charmbreaker Devils", P0);

    // Put an instant spell on the stack and fire SpellCast event.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let spell = state.create_object(bolt_id, P0, Zone::Stack, None, None);
    state.get_object_mut(spell).unwrap().name = "Lightning Bolt".into();
    state.events.push(mtg_engine::events::GameEvent::SpellCast {
        player: P0,
        object: spell,
    });
    triggers::process_triggers(&mut state, &reg);

    let power = state.effective_power(devils, &reg).unwrap();
    assert_eq!(power, 8, "Charmbreaker Devils should be 4+4=8 power after spell cast");
}

// ── Curse of the Bloody Tome ──────────────────────────────────────

/// Curse mills 2 from enchanted player on their upkeep.
#[test]
fn curse_of_bloody_tome_mills_on_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Bloody Tome", P0, P1);

    stock_library(&mut state, &reg, P1, 4);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let gy = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert_eq!(gy, 2, "Should mill 2 cards from P1's library");
}

// ── Curse of Oblivion ─────────────────────────────────────────────

/// Curse exiles 2 cards from enchanted player's graveyard (auto when ≤2).
#[test]
fn curse_of_oblivion_exiles_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Oblivion", P0, P1);

    // Put 2 cards in P1's graveyard (auto-exiles when ≤2).
    let g1 = state.create_object(CardId(9999), P1, Zone::Graveyard, None, None);
    let g2 = state.create_object(CardId(9999), P1, Zone::Graveyard, None, None);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(g1).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(g2).unwrap().zone, Zone::Exile);
}

/// Ruling: "If the enchanted player has only one card in their graveyard, they
/// exile that card." Not "no cards are exiled" — the effect does as much as it
/// can (CR 608.2).
#[test]
fn curse_of_oblivion_exiles_the_only_card_when_there_is_just_one() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);
    attach_curse_to_player(&mut state, &reg, "Curse of Oblivion", P0, P1);

    let only = state.create_object(CardId(9999), P1, Zone::Graveyard, None, None);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(only).unwrap().zone, Zone::Exile,
        "the one card is exiled");
}

/// An empty graveyard is not an error — nothing is exiled and no choice is
/// offered.
#[test]
fn curse_of_oblivion_does_nothing_with_an_empty_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);
    attach_curse_to_player(&mut state, &reg, "Curse of Oblivion", P0, P1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(state.awaiting_action.is_none(), "no card to choose, so no prompt");
}

/// With three or more cards the enchanted player chooses which two to exile,
/// one prompt at a time. This is the branch that carries a countdown in the
/// effect key, and it had no test — an off-by-one there would exile one card or
/// three instead of two.
#[test]
fn curse_of_oblivion_lets_the_cursed_player_choose_exactly_two_of_several() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);
    attach_curse_to_player(&mut state, &reg, "Curse of Oblivion", P0, P1);

    let g: Vec<ObjectId> = (0..4)
        .map(|_| state.create_object(CardId(9999), P1, Zone::Graveyard, None, None))
        .collect();

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // First prompt — and it goes to the *cursed* player, not the Curse's
    // controller.
    match state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, .. }) =>
            assert_eq!(player, P1, "the enchanted player chooses"),
        _ => panic!("expected a choice of card to exile"),
    }
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(
                Some(mtg_engine::actions::Target::Object(g[0]))) },
        &reg,
    );

    // Second prompt.
    assert!(state.awaiting_action.is_some(), "a second card is still owed");
    state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(
                Some(mtg_engine::actions::Target::Object(g[1]))) },
        &reg,
    );

    assert!(state.awaiting_action.is_none(), "two is two — no third prompt");
    let exiled = g.iter().filter(|&&id| state.get_object(id).unwrap().zone == Zone::Exile).count();
    assert_eq!(exiled, 2, "exactly two cards left the graveyard");
    assert_eq!(state.get_object(g[0]).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(g[1]).unwrap().zone, Zone::Exile);
}

// ── Curse of the Nightly Hunt ─────────────────────────────────────

/// Curse forces enchanted player's creatures to attack.
#[test]
fn curse_of_nightly_hunt_forces_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P1); // P1's turn

    // P0 controls curse attached to P1.
    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Nightly Hunt", P0, P1);

    // P1's creature should be forced to attack.
    let creature = ready_creature(&mut state, P1, 2, 2);

    let has_force = state.has_effect(creature,
        &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), &reg);
    assert!(has_force, "P1's creature should be forced to attack by curse");

    // P0's creature should NOT be forced.
    let own_creature = ready_creature(&mut state, P0, 2, 2);
    let own_forced = state.has_effect(own_creature,
        &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), &reg);
    assert!(!own_forced, "P0's creature should NOT be forced to attack");
}

// ── Whose upkeep a Curse watches (CR 603.2) ───────────────────────

/// "At the beginning of enchanted player's upkeep" — not the controller's.
/// Each of these Curses is controlled by P0 and attached to P1, so P0's own
/// upkeep must do nothing at all.
///
/// The per-Curse tests above all fire on the enchanted player's upkeep and so
/// would pass equally for a Curse that triggered on *every* upkeep. This is
/// the half that separates them.
#[test]
fn a_curse_does_nothing_on_its_controllers_upkeep() {
    const CURSES: &[&str] = &[
        "Curse of the Pierced Heart",
        "Curse of the Bloody Tome",
        "Curse of Oblivion",
    ];

    for name in CURSES {
        let reg = registry();
        // P0's upkeep — the Curse's controller, not the enchanted player.
        let mut state = game_at_step(Step::Upkeep, P0);
        attach_curse_to_player(&mut state, &reg, name, P0, P1);

        stock_library(&mut state, &reg, P1, 4);
        let graveyard = [
            state.create_object(CardId(9999), P1, Zone::Graveyard, None, None),
            state.create_object(CardId(9999), P1, Zone::Graveyard, None, None),
        ];
        let library_before = state.get_player(P1).library_order.len();

        fire_step_trigger(&mut state, Step::Upkeep, &reg);

        assert_eq!(state.get_player(P1).life, 20,
            "{name}: no damage on its controller's upkeep");
        assert_eq!(state.get_player(P1).library_order.len(), library_before,
            "{name}: no mill on its controller's upkeep");
        for id in graveyard {
            assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard,
                "{name}: nothing exiled on its controller's upkeep");
        }
    }
}

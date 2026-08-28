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
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
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

/// Ruling 2025-01-24: "If Splinterfright's controller has only one card in
/// their library when its triggered ability resolves, they put that card into
/// their graveyard." Milling more cards than a library holds mills all of them
/// and does not lose the game for the shortfall (CR 701.13b); the loss comes
/// later, from trying to draw from an empty library.
#[test]
fn splinterfright_mills_what_is_left_of_a_short_library() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _splinter = named_permanent(&mut state, &reg, "Splinterfright", P0);
    stock_library(&mut state, &reg, P0, 1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_player(P0).library_order.len(), 0, "the one card went");
    assert_eq!(
        state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == P0).count(),
        1,
        "one card, not two, and no panic over the missing second");
    assert!(!state.get_player(P0).lost,
        "milling an empty library is not a loss (CR 701.13b) — drawing from one is");
}

/// An empty library is not an error either: nothing to mill, nothing happens.
#[test]
fn splinterfright_against_an_empty_library_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let _splinter = named_permanent(&mut state, &reg, "Splinterfright", P0);
    assert_eq!(state.get_player(P0).library_order.len(), 0, "test setup");

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(!state.get_player(P0).lost);
    assert_eq!(
        state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == P0).count(),
        0);
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

    // The trigger goes on the stack and its target is chosen (CR 603.3d).
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

    // CR 603.3d: the target is chosen as the trigger goes on the stack, so the
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

/// Put `n` 2/2 Zombie tokens onto the battlefield under `player`.
fn zombies_for(state: &mut mtg_engine::state::GameState, reg: &CardRegistry, player: PlayerId, n: usize) {
    for _ in 0..n {
        let z = state.create_token_with_subtypes(
            "Zombie", player, 2, 2, vec![Color::Black],
            vec![CardType::Creature], vec![], vec!["Zombie".into()], reg)[0];
        state.get_object_mut(z).unwrap().summoning_sick = false;
    }
}

fn zombies_controlled_by(state: &mtg_engine::state::GameState, reg: &CardRegistry, player: PlayerId) -> usize {
    state.objects_in_zone(Zone::Battlefield, player).iter()
        .filter(|o| state.has_subtype(o.id, "Zombie", reg))
        .count()
}

/// "X is half the number of Zombies **you control**, rounded down."
///
/// Ruling: "If you control fewer than two Zombies, you won't get any tokens."
/// The existing test covers 5 → 2 only, which a great many wrong formulas also
/// satisfy; and it has no opponent on the board, so nothing pinned "you
/// control".
#[test]
fn endless_ranks_makes_half_your_zombies_rounded_down() {
    let reg = registry();
    // (your Zombies, opponent's Zombies, tokens expected)
    for (mine, theirs, expected) in [(0, 4, 0), (1, 4, 0), (2, 0, 1), (3, 0, 1), (4, 0, 2), (7, 6, 3)] {
        let mut state = game_at_step(Step::Upkeep, P0);
        named_permanent(&mut state, &reg, "Endless Ranks of the Dead", P0);
        zombies_for(&mut state, &reg, P0, mine);
        zombies_for(&mut state, &reg, P1, theirs);

        fire_step_trigger(&mut state, Step::Upkeep, &reg);

        assert_eq!(zombies_controlled_by(&state, &reg, P0), mine + expected,
            "{mine} Zombies of yours and {theirs} of theirs should make {expected} tokens");
        assert_eq!(zombies_controlled_by(&state, &reg, P1), theirs,
            "and none of them arrive under the opponent");
    }
}

/// "create X **2/2 black Zombie** creature tokens." The count test above
/// would be satisfied by tokens of any size or colour.
#[test]
fn endless_ranks_tokens_are_two_two_black_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    named_permanent(&mut state, &reg, "Endless Ranks of the Dead", P0);

    zombies_for(&mut state, &reg, P0, 2);

    let existing: std::collections::HashSet<ObjectId> =
        state.objects.values().map(|o| o.id).collect();
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let made: Vec<ObjectId> = state.objects.values()
        .filter(|o| !existing.contains(&o.id))
        .map(|o| o.id)
        .collect();
    assert_eq!(made.len(), 1, "two Zombies make one token");
    let token = made[0];
    assert_eq!(state.effective_power(token, &reg), Some(2), "2/2");
    assert_eq!(state.effective_toughness(token, &reg), Some(2), "2/2");
    assert_eq!(state.get_object(token).unwrap().colors, vec![Color::Black], "black");
    assert!(state.has_subtype(token, "Zombie", &reg), "Zombie");
    assert!(state.get_object(token).unwrap().is_token, "a token");
}

/// Ruling: "The number of Zombies you control is counted when the ability
/// resolves. If you control multiple Endless Ranks of the Dead, the tokens you
/// get when the first ability resolves will count for the subsequent abilities."
///
/// Four Zombies to start. Counted at resolution: 4/2 = 2 tokens, then 6/2 = 3
/// more, for nine Zombies. Counted once up front, both abilities would see 4
/// and make 2 each, for eight. The numbers are chosen so those differ.
#[test]
fn a_second_endless_ranks_counts_the_tokens_the_first_one_made() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    named_permanent(&mut state, &reg, "Endless Ranks of the Dead", P0);
    named_permanent(&mut state, &reg, "Endless Ranks of the Dead", P0);
    zombies_for(&mut state, &reg, P0, 4);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(zombies_controlled_by(&state, &reg, P0), 9,
        "4 -> +2 -> 6 -> +3 -> 9; eight would mean both abilities counted the \
         same four Zombies");
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

/// "…to that player **or** a planeswalker that player controls" — the choice
/// is offered by position, so the options must be in a fixed order. Built from
/// `state.objects.values()` they came out in HashMap order, which is seeded
/// per process: the same game replayed twice offered the same planeswalkers
/// under different indices.
#[test]
fn curse_of_pierced_heart_offers_its_options_in_a_stable_order() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let lily = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P1);
    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseTarget { options, .. }, ..
    }) = &state.awaiting_action else {
        panic!("the Curse must ask which of the two planeswalkers, or the \
                player, takes the damage; got {:?}", state.awaiting_action);
    };
    let _ = curse;
    let mut expected = vec![Target::Object(lily), Target::Object(garruk)];
    expected.sort_by_key(|t| match t { Target::Object(id) => id.0, _ => 0 });
    assert_eq!(options[0], Target::Player(P1),
        "the enchanted player is the first option");
    assert_eq!(&options[1..], &expected[..],
        "the planeswalkers must be offered in object-id order, not map order");
}

/// CR 608.2g: the ability is controlled by whoever controlled the Curse when
/// it triggered. Leaving the battlefield resets `controller` to `owner`, so a
/// Curse destroyed in response to its own trigger (CR 113.7a) must not hand
/// the choice to its owner.
#[test]
fn curse_of_pierced_heart_asks_its_last_controller_after_it_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let _lily = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    // Owned by P1, controlled by P0 — the two differ, which is the only way to
    // tell which of them the code read.
    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);
    state.get_object_mut(curse).unwrap().owner = P1;

    // Destroyed with the trigger already on the stack.
    state.move_object(curse, Zone::Graveyard, &reg);

    let behavior = reg.get(state.get_object(curse).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, curse, &[], &reg);

    let Some(AwaitingAction::ResolutionChoice { player, .. }) = &state.awaiting_action else {
        panic!("the trigger still resolves after the Curse is destroyed; \
                got {:?}", state.awaiting_action);
    };
    assert_eq!(*player, P0,
        "the player who controlled the Curse chooses, not its owner");
}

// ── Curse of Death's Hold ─────────────────────────────────────────

/// "Creatures enchanted player controls get -1/-1."
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

/// "**enchanted player**", not "your opponents". "Enchant player" names any
/// player, and a Curse can be put on the player who controls it — at which
/// point the -1/-1 lands on that player's own creatures and the opponent's are
/// untouched. This is the board on which "the cursed player" and "everyone who
/// isn't me" give opposite answers, and it is the only board that can tell
/// which one the code asked.
#[test]
fn curse_of_deaths_hold_debuffs_its_own_controller_when_it_enchants_them() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Death's Hold", P0, P0);

    let cursed = ready_creature(&mut state, P0, 3, 3);
    let uncursed = ready_creature(&mut state, P1, 3, 3);

    assert_eq!(state.effective_power(cursed, &reg), Some(2),
        "the enchanted player's creature shrinks, though they control the Curse");
    assert_eq!(state.effective_toughness(cursed, &reg), Some(2));
    assert_eq!(state.effective_power(uncursed, &reg), Some(3),
        "and the opponent's does not");
}

/// The point of the card: -1/-1 puts an X/1 into the graveyard as a
/// state-based action (CR 704.5f), and takes that back the moment the Curse
/// leaves the battlefield.
#[test]
fn curse_of_deaths_hold_kills_one_toughness_creatures_and_stops_when_it_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of Death's Hold", P0, P1);
    let flier = ready_creature(&mut state, P1, 2, 1);
    let survivor = ready_creature(&mut state, P1, 2, 2);

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(flier).unwrap().zone, Zone::Graveyard,
        "0 toughness, so it dies to SBA — no damage and no destruction involved");
    assert_eq!(state.get_object(survivor).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.effective_toughness(survivor, &reg), Some(1));

    state.move_object(curse, Zone::Graveyard, &reg);
    assert_eq!(state.effective_toughness(survivor, &reg), Some(2),
        "a continuous effect lasts only while its source is on the battlefield");
}

/// The -1/-1 applies on top of a characteristic-defining base, not instead of
/// it: Boneyard Wurm's "power and toughness are each equal to the number of
/// creature cards in your graveyard" is layer 7a and the Curse is 7c, so a
/// Wurm with three creature cards under a Curse is a 2/2.
#[test]
fn curse_of_deaths_hold_subtracts_from_a_defined_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Death's Hold", P0, P1);
    let wurm = named_permanent(&mut state, &reg, "Boneyard Wurm", P1);
    for name in ["Walking Corpse", "Grizzly Bears", "Chapel Geist"] {
        named_card_in_graveyard(&mut state, &reg, name, P1);
    }

    assert_eq!(state.effective_power(wurm, &reg), Some(2),
        "three creature cards in the graveyard, less the Curse's -1");
    assert_eq!(state.effective_toughness(wurm, &reg), Some(2));
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

/// Ruling 2011-09-22: "The Spirit card must already be in your graveyard when
/// the ability triggers at the beginning of your upkeep. If there is no Spirit
/// card in your graveyard when your upkeep begins, the ability will be removed
/// from the stack with no effect."
///
/// That is CR 603.3c, and it is deliberately *not* CR 603.4: the ability does
/// trigger and does go on the stack, and is then removed for want of targets.
/// Morkrut Banshee's morbid is the other case — its ruling says the ability
/// "won't trigger at all" — and the two are implemented differently because
/// the rulings say different things.
#[test]
fn angel_of_flight_alabaster_is_removed_from_the_stack_with_no_spirit() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    // A card in the graveyard, but not a Spirit.
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(state.stack.is_empty(), "nothing is left on the stack");
    let removed: Vec<&str> = state.game_log.iter()
        .filter(|e| e.message.contains("Trigger removed"))
        .map(|e| e.message.as_str())
        .collect();
    assert_eq!(removed.len(), 1,
        "the ability triggered and was then removed for want of a legal target \
         (CR 603.3c), which is what the ruling describes; log was {:?}",
        state.game_log.iter().map(|e| &e.message).collect::<Vec<_>>());
}

/// "target Spirit **card**" — a Spirit token that died is in the graveyard
/// until the next state-based-action check, and is not a card (CR 109.1).
/// Doomed Traveler makes one.
#[test]
fn angel_of_flight_alabaster_cannot_target_a_spirit_token() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let token = state.create_token_with_subtypes(
        "", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying],
        vec!["Spirit".into()], &reg)[0];
    state.move_object(token, Zone::Graveyard, &reg);
    assert!(state.get_object(token).is_some(), "test premise: it is still there");
    assert!(state.has_subtype(token, "Spirit", &reg), "and it is a Spirit");

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(token).unwrap().zone, Zone::Graveyard,
        "a Spirit token is not a Spirit card, so there was nothing to return");
    assert!(state.stack.is_empty());
}

/// "from **your** graveyard" — an opponent's Spirit card is not a candidate.
#[test]
fn angel_of_flight_alabaster_cannot_reach_an_opponents_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P1);
    let mine = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Hand,
        "your own Spirit card is the one legal target, so it is taken");
    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard,
        "an opponent's Spirit card is not in your graveyard");
}

// ── Charmbreaker Devils ───────────────────────────────────────────

/// "At the beginning of your upkeep, return an instant or sorcery card at
/// random from your graveyard to your hand." The card's other half, which had
/// no test at all.
#[test]
fn charmbreaker_devils_returns_an_instant_or_sorcery_at_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    named_permanent(&mut state, &reg, "Charmbreaker Devils", P0);

    // One instant and one creature card in the graveyard: only the instant is
    // a candidate, so the random pick is deterministic here.
    let bolt = state.create_object(
        reg.get_id_by_name("Brimstone Volley").unwrap(), P0, Zone::Graveyard, None, None);
    let creature = state.create_object(
        reg.get_id_by_name("Ambush Viper").unwrap(), P0, Zone::Graveyard, None, None);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Hand,
        "the instant came back");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "the creature card is not a candidate");
}

/// Nothing to return is not an error.
#[test]
fn charmbreaker_devils_does_nothing_with_no_instants_or_sorceries() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    named_permanent(&mut state, &reg, "Charmbreaker Devils", P0);

    let creature = state.create_object(
        reg.get_id_by_name("Ambush Viper").unwrap(), P0, Zone::Graveyard, None, None);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// "At the beginning of **your** upkeep" — an opponent's Devils do not return a
/// card on your turn.
#[test]
fn charmbreaker_devils_returns_nothing_on_the_opponents_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    named_permanent(&mut state, &reg, "Charmbreaker Devils", P1);

    let bolt = state.create_object(
        reg.get_id_by_name("Brimstone Volley").unwrap(), P1, Zone::Graveyard, None, None);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard,
        "it is P0's upkeep, not the Devils' controller's");
}

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

/// Ruling: "If the enchanted player has only one card in their library, they
/// put that card into their graveyard."
///
/// Milling is not drawing: a short library mills what is there (CR 701.13b)
/// and nobody loses. The log has to say what happened rather than what was
/// asked for — six cards used to log their intended count beside
/// `mill_cards`'s real one.
#[test]
fn curse_of_bloody_tome_mills_the_last_card_and_says_so() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of the Bloody Tome", P0, P1);
    let only = stock_library(&mut state, &reg, P1, 1)[0];

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert_eq!(state.get_object(only).unwrap().zone, Zone::Graveyard,
        "the one card in the library is milled");
    assert!(state.get_player(P1).library_order.is_empty(), "and the library is empty");
    assert!(!state.get_player(P1).lost,
        "milling an empty library is not drawing from one — nobody loses");

    let mill_lines: Vec<&str> = state.game_log.iter()
        .filter(|e| e.message.contains("milled"))
        .map(|e| e.message.as_str())
        .collect();
    assert_eq!(mill_lines.len(), 1, "one line for one mill; got {mill_lines:?}");
    assert!(mill_lines[0].contains("Curse of the Bloody Tome"),
        "and it names the source: {mill_lines:?}");
    assert!(mill_lines[0].contains("1 card") && mill_lines[0].contains("of 2"),
        "and reports what happened, not what was asked for: {mill_lines:?}");
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

//! CR 400.7: a permanent that changes zones becomes a NEW object with no
//! memory of the old one. Anything an effect changed about it while it was on
//! the battlefield has to be gone.
//!
//! `move_object` reset the obviously-battlefield things (tapped, damage,
//! counters, attachment to a permanent) but left behind everything an effect
//! had rewritten about the object's identity: a copy's `card_id`, an exchanged
//! base toughness, a Curse's attachment to a player, the display name. Each
//! produced a wrong result the moment the card came back.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use mtg_engine::sba::check_state_based_actions;

/// A copy stops being a copy. Evil Twin in the graveyard is an Evil Twin —
/// not the creature it copied — so reanimating it runs its OWN enters
/// handler and it can offer the copy choice again.
#[test]
fn a_copy_reverts_to_its_printed_card_on_leaving_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let victim = named_permanent(&mut state, &reg, "Bloodgift Demon", P1);

    // Resolve the copy the way the ETB choice does.
    mtg_engine::engine::apply_pending_effect(
        &mut state,
        &mtg_engine::actions::Target::Object(victim),
        &mtg_engine::state::PendingEffect::CopyCreature { source_id: twin },
        &reg,
    );
    assert_eq!(state.name_of(twin, &reg), "Bloodgift Demon", "test precondition: it copied");

    state.move_object(twin, Zone::Graveyard, &reg);

    assert_eq!(state.name_of(twin, &reg), "Evil Twin",
        "in the graveyard it is an Evil Twin again, not the creature it copied");
    assert_eq!(state.get_object(twin).unwrap().name, "Evil Twin",
        "the display name reverts too");
    assert!(state.get_object(twin).unwrap().copy_grantor.is_none(),
        "it is no longer a copy of anything");
}

/// Tree of Redemption swaps its toughness with its controller's life total.
/// That swap must not follow it into the graveyard and back.
#[test]
fn an_exchanged_base_toughness_does_not_survive_a_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    let printed = state.get_object(tree).unwrap().toughness;
    assert_eq!(printed, Some(13), "test precondition: Tree of Redemption is 0/13");

    // The exchange writes the base toughness directly.
    state.get_object_mut(tree).unwrap().toughness = Some(4);

    state.move_object(tree, Zone::Graveyard, &reg);

    assert_eq!(state.get_object(tree).unwrap().toughness, printed,
        "the graveyard object is a new object printed as 0/13, not the 0/4 the \
         exchange left behind");
}

/// A Curse attached to a player must not stay attached once it leaves.
#[test]
fn a_curse_detaches_from_its_player_on_leaving_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Pierced Heart", P0, P1);
    assert_eq!(state.get_object(curse).unwrap().attached_to_player, Some(P1),
        "test precondition");

    state.move_object(curse, Zone::Graveyard, &reg);

    assert_eq!(state.get_object(curse).unwrap().attached_to_player, None,
        "otherwise any effect returning the Curse to the battlefield would \
         re-attach it to that player with no targeting and no consent");
}

/// A token copy of a double-faced card is not itself double-faced, so it
/// cannot transform — even though it carries the DFC's card_id and would
/// otherwise pick up its upkeep trigger.
#[test]
fn a_token_copy_of_a_werewolf_cannot_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);
    let token = state.create_token_copy(waif, P0, &reg);
    assert!(state.get_object(token).unwrap().is_token, "test precondition");

    // No spells were cast last turn, so a real Reckless Waif would flip.
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(state.get_object(waif).unwrap().is_transformed,
        "test precondition: the real card transforms under these conditions");
    assert!(!state.get_object(token).unwrap().is_transformed,
        "a token copy has only the copied face and cannot transform");

    // And it must not say it did. Each werewolf used to log the flip itself,
    // around the call that refuses one for a token — so the log announced two
    // transforms where only the real card flipped. The announcement now lives
    // inside `apply_transform`, on the path that actually flips.
    let announced = state.game_log.iter()
        .filter(|e| e.message.contains("transforms into"))
        .count();
    assert_eq!(announced, 1,
        "exactly one transform happened, so the log must report exactly one; \
         got: {:?}",
        state.game_log.iter()
            .filter(|e| e.message.contains("transforms into"))
            .map(|e| e.message.clone()).collect::<Vec<_>>());
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// CR 400.7: a permanent that leaves the battlefield and comes back is a new
/// object, so nothing it accumulated comes with it. For Ludevic's Test Subject
/// that means its hatchling counters: killed at four counters and reanimated,
/// it starts over at zero rather than one activation from a 13/13.
///
/// This used to hand-insert a `card_state["hatchling_counters"]` key, back when
/// the card kept its own tally there instead of using real counters. The key
/// has not existed since; the test asserted that a map nobody writes to is
/// empty. Both halves are worth checking — the counters *and* card_state — so
/// check them on the state the card actually keeps.
#[test]
fn a_reanimated_permanent_brings_back_neither_counters_nor_card_state() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let subject = named_permanent(&mut state, &registry, "Ludevic's Test Subject", P0);

    // Four activations short of the flip.
    state.add_counters(subject, CounterType::Hatchling, 4);
    state.get_object_mut(subject).unwrap()
        .card_state.insert("scratch".into(), mtg_engine::ids::ObjectId(3));
    assert_eq!(state.get_counter_count(subject, CounterType::Hatchling), 4);

    // It dies and is reanimated.
    state.move_object(subject, Zone::Graveyard, &registry);
    state.move_object(subject, Zone::Battlefield, &registry);

    assert_eq!(state.get_counter_count(subject, CounterType::Hatchling), 0,
        "counters do not survive the trip through the graveyard (CR 121.2)");
    assert!(state.get_object(subject).unwrap().card_state.is_empty(),
        "and neither does anything else the old object was carrying");
}

// -------------------------------------------------------------------------
// What a zone change leaves behind
// -------------------------------------------------------------------------

/// When a creature leaves the battlefield and re-enters, attached auras
/// should fall off. The re-entered creature is a "new object" per rule 400.7.
#[test]
fn blinked_creature_loses_aura() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);

    // Attach an aura.
    let aura_id = reg.get_id_by_name("Holy Strength").unwrap();
    let aura = state.create_object(aura_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().attached_to = Some(creature);

    // "Blink" the creature: exile then return to battlefield.
    state.move_object(creature, Zone::Exile, &reg);

    // SBA should clean up the aura (its target left the battlefield).
    check_state_based_actions(&mut state, &reg);

    assert_eq!(
        state.get_object(aura).unwrap().zone,
        Zone::Graveyard,
        "Aura should fall off when creature is blinked (exiled) — rule 400.7"
    );
}

/// When a creature leaves and re-enters, damage should be cleared.
#[test]
fn zone_change_clears_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    // Mark some damage.
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    // Move to hand and back to battlefield.
    state.move_object(creature, Zone::Hand, &reg);
    state.move_object(creature, Zone::Battlefield, &reg);

    assert_eq!(
        state.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be cleared when a creature re-enters the battlefield"
    );
}

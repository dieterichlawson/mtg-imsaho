//! CR 608.2b: when a triggered ability tries to resolve, its targets are
//! re-checked. If they have all become illegal, the ability is countered by
//! the game rules.
//!
//! The re-check ran only the generic half — zone, hexproof, target filter —
//! and skipped `is_valid_target`, the card's own restriction on what it may
//! target. `resolve_spell` had always run both. So a trigger resolved happily
//! against a target that had stopped satisfying the card's wording: Grimgrin's
//! "creature the defending player controls" survived that creature changing
//! controller in response.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::AttackInfo;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// Angel of Flight Alabaster targets "a Spirit card in your graveyard". A
/// card that stops being a legal target between announcement and resolution
/// makes the ability fizzle.
#[test]
fn a_trigger_fizzles_when_its_target_stops_satisfying_the_cards_restriction() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    // A card in the graveyard that is NOT a Spirit. It satisfies the generic
    // half of legality — right zone, no hexproof, matches the target filter —
    // and is rejected only by the card's own `is_valid_target` ("target Spirit
    // card"). That is precisely the half the re-check used to skip.
    let not_a_spirit = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert!(!state.has_subtype(not_a_spirit, "Spirit", &reg), "test precondition");

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(not_a_spirit)], ..TriggerSource::new(angel, angel_card, P0, "Angel of Flight Alabaster") },
        event: TriggerEvent::Upkeep,
    }));
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(not_a_spirit).unwrap().zone, Zone::Graveyard,
        "the only target does not satisfy 'target Spirit card', so the ability \
         is countered on resolution rather than returning it (CR 608.2b)");
}

/// The happy path still works: a legal target is still returned.
#[test]
fn a_trigger_with_a_still_legal_target_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(spirit)], ..TriggerSource::new(angel, angel_card, P0, "Angel of Flight Alabaster") },
        event: TriggerEvent::Upkeep,
    }));
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "a legal Spirit card in the graveyard is returned to hand");
}

/// Civilized Scholar's "unless it attacked this turn" marker is stamped with
/// the turn it happened on. A bare marker could not be told apart from one
/// left over from a previous turn, and the clearing path only ran on the back
/// face's end step — so a front-face attack in turn N stuck forever and
/// stopped the Brute transforming back in every later turn.
#[test]
fn an_attack_in_an_earlier_turn_does_not_keep_the_brute_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let scholar = named_permanent(&mut state, &reg, "Civilized Scholar", P0);
    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();

    // It attacked on the front face this turn...
    behavior.on_attacks(&mut state, scholar, AttackInfo::new(scholar, P1), &[], &reg);
    // ...then a later turn begins, and it transforms.
    state.turn_number += 1;
    mtg_engine::cards::helpers::apply_transform(&mut state, scholar, &reg);
    assert!(state.get_object(scholar).unwrap().is_transformed, "test precondition");

    behavior.on_end_step(&mut state, scholar, &[], &reg);

    assert!(!state.get_object(scholar).unwrap().is_transformed,
        "the attack was in a PREVIOUS turn, so Homicidal Brute did not attack \
         this turn and must tap and transform back");
}

/// The same-turn case still holds: an attack this turn keeps it transformed
/// (CR 711.5 — transforming does not make a new object).
#[test]
fn an_attack_this_turn_keeps_the_brute_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let scholar = named_permanent(&mut state, &reg, "Civilized Scholar", P0);
    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();

    behavior.on_attacks(&mut state, scholar, AttackInfo::new(scholar, P1), &[], &reg);
    mtg_engine::cards::helpers::apply_transform(&mut state, scholar, &reg);
    behavior.on_end_step(&mut state, scholar, &[], &reg);

    assert!(state.get_object(scholar).unwrap().is_transformed,
        "it attacked this turn, so it stays a Homicidal Brute");
}

// -------------------------------------------------------------------------
// Per-card cases
// -------------------------------------------------------------------------

// CR 608.2b: When a triggered ability with targets resolves, the game re-checks
// each target for legality. If all targets are illegal, the ability is countered.
// Angel of Flight Alabaster upkeep trigger targets a Spirit in the graveyard.
// Exiling that Spirit in response makes the graveyard target zone-illegal,
// so the trigger should fizzle and the Spirit should stay in exile.
#[test]
fn test_angel_of_flight_alabaster_fizzles_on_illegal_target() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    let spirit = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(spirit).unwrap().subtypes.push("Spirit".into());
    state.move_object(spirit, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(spirit)], ..TriggerSource::new(angel, angel_card, P0, "Angel of Flight Alabaster") },
        event: TriggerEvent::Upkeep,
    }));

    // In response: exile the Spirit (e.g., Purify the Grave)
    state.move_object(spirit, Zone::Exile, &reg);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(spirit).unwrap().zone,
        Zone::Exile,
        "CR 608.2b: trigger should fizzle when graveyard target is exiled — Spirit must stay in exile"
    );
}

// CR 608.2b: Grimgrin attack trigger targets a creature for destruction plus
// a +1/+1 counter on Grimgrin. Granting the target hexproof in response makes
// it an illegal target (opponent-controlled creature with hexproof), so the
// trigger should fizzle: no destruction AND no counter.
#[test]
fn test_grimgrin_attack_trigger_fizzles_on_illegal_target() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let grimgrin_card = reg.get_id_by_name("Grimgrin, Corpse-Born").unwrap();

    let target_creature = ready_creature(&mut state, P1, 3, 3);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(target_creature)], ..TriggerSource::new(grimgrin, grimgrin_card, P0, "Grimgrin, Corpse-Born") },
        event: TriggerEvent::Attacks { attacker: grimgrin, defending_player: P1 },
    }));

    // In response: give the target hexproof
    state.get_object_mut(target_creature).unwrap().keywords.push(Keyword::Hexproof);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(target_creature).unwrap().zone,
        Zone::Battlefield,
        "CR 608.2b: trigger should fizzle on hexproof target — creature must not be destroyed"
    );
    assert_eq!(
        counters_of(&state, grimgrin, CounterType::PlusOnePlusOne),
        0,
        "CR 608.2b: trigger should fizzle on hexproof target — no +1/+1 counter on Grimgrin"
    );
}

// CR 608.2b: Morkrut Banshee ETB (morbid) targets a creature for -4/-4.
// Granting the target hexproof in response makes it illegal (opponent-controlled
// creature with hexproof can't be targeted), so the trigger should fizzle and
// the -4/-4 modifier should not be applied.
#[test]
fn test_morkrut_banshee_fizzles_on_illegal_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let banshee = named_permanent(&mut state, &reg, "Morkrut Banshee", P0);
    let banshee_card = reg.get_id_by_name("Morkrut Banshee").unwrap();

    let target_creature = ready_creature(&mut state, P1, 4, 4);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(target_creature)], ..TriggerSource::new(banshee, banshee_card, P0, "Morkrut Banshee") },
        event: TriggerEvent::SelfEntered,
    }));

    // In response: give the target hexproof (making it an illegal target
    // for the opponent-controlled trigger per CR 702.11b)
    state.get_object_mut(target_creature).unwrap().keywords.push(Keyword::Hexproof);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.effective_power(target_creature, &reg),
        Some(4),
        "CR 608.2b: trigger should fizzle on hexproof target — no -4/-4 modifier applied"
    );
}

// CR 608.2b: Snapcaster Mage ETB targets an instant/sorcery in the graveyard
// to grant flashback. Exiling the instant in response makes the graveyard
// target zone-illegal, so the trigger should fizzle and no flashback is granted.
#[test]
fn test_snapcaster_trigger_fizzles_if_target_exiled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let snapcaster = named_permanent(&mut state, &reg, "Snapcaster Mage", P0);
    let snapcaster_card = reg.get_id_by_name("Snapcaster Mage").unwrap();

    let instant = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(instant)], ..TriggerSource::new(snapcaster, snapcaster_card, P0, "Snapcaster Mage") },
        event: TriggerEvent::SelfEntered,
    }));

    // In response: exile the instant
    state.move_object(instant, Zone::Exile, &reg);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    let flashback_count = state.until_end_of_turn.iter()
        .filter(|e| matches!(e,
            mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == instant))
        .count();
    assert_eq!(
        flashback_count, 0,
        "CR 608.2b: trigger should fizzle when graveyard target is exiled — no flashback granted"
    );
}

// CR 608.2b: Reaper from the Abyss end-step trigger targets a non-Demon creature
// for destruction. Bouncing the target to hand makes it zone-illegal (must be
// on the battlefield for TargetRequirement::Creature), so the trigger should
// fizzle and the creature should remain in hand.
#[test]
fn reaper_target_bounced_before_resolve() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    state.creature_died_this_turn = true;

    let reaper = named_permanent(&mut state, &reg, "Reaper from the Abyss", P0);
    let reaper_card = reg.get_id_by_name("Reaper from the Abyss").unwrap();

    let creature = ready_creature(&mut state, P1, 3, 3);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(creature)], ..TriggerSource::new(reaper, reaper_card, P0, "Reaper from the Abyss") },
        event: TriggerEvent::EndStep,
    }));

    // In response: bounce the target to hand
    state.move_object(creature, Zone::Hand, &reg);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(creature).unwrap().zone,
        Zone::Hand,
        "CR 608.2b: trigger should fizzle when target is bounced — creature must remain in hand"
    );
}

// CR 608.2b: Reaper from the Abyss end-step trigger targets a non-Demon creature.
// If the target gains the Demon subtype before resolution (e.g., via Conspiracy),
// it no longer satisfies the "non-Demon" targeting restriction and the trigger
// should fizzle via is_valid_target re-check.
#[test]
fn reaper_target_becomes_demon_before_resolve() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    state.creature_died_this_turn = true;

    let reaper = named_permanent(&mut state, &reg, "Reaper from the Abyss", P0);
    let reaper_card = reg.get_id_by_name("Reaper from the Abyss").unwrap();

    let creature = ready_creature(&mut state, P1, 3, 3);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(creature)], ..TriggerSource::new(reaper, reaper_card, P0, "Reaper from the Abyss") },
        event: TriggerEvent::EndStep,
    }));

    // In response: target becomes a Demon (e.g., via Conspiracy)
    state.get_object_mut(creature).unwrap().subtypes.push("Demon".into());

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(creature).unwrap().zone,
        Zone::Battlefield,
        "CR 608.2b: trigger should fizzle when target becomes a Demon — creature must survive"
    );
}

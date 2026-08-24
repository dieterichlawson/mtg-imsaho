mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// CR 608.2b: When a triggered ability with targets resolves, the game re-checks
// each target for legality. If all targets are illegal, the ability is countered.
// Angel of Flight Alabaster upkeep trigger targets a Spirit in the graveyard.
// Exiling that Spirit in response makes the graveyard target zone-illegal,
// so the trigger should fizzle and the Spirit should stay in exile.
#[test]
fn test_angel_of_flight_alabaster_fizzles_on_illegal_target() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    let spirit = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(spirit).unwrap().subtypes.push("Spirit".into());
    state.move_object(spirit, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: angel,
        card_id: angel_card,
        controller: P0,
        description: "Angel of Flight Alabaster".into(),
        chosen_targets: vec![Target::Object(spirit)],
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

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let grimgrin_card = reg.get_id_by_name("Grimgrin, Corpse-Born").unwrap();

    let target_creature = ready_creature(&mut state, P1, 3, 3);

    state.stack.push(StackEntry::Trigger(PendingTrigger::AttacksTrigger {
        object_id: grimgrin,
        card_id: grimgrin_card,
        controller: P0,
        description: "Grimgrin, Corpse-Born".into(),
        attacker: grimgrin,
        defending_player: P1,
        chosen_targets: vec![Target::Object(target_creature)],
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

    let banshee = named_creature(&mut state, &reg, "Morkrut Banshee", P0);
    let banshee_card = reg.get_id_by_name("Morkrut Banshee").unwrap();

    let target_creature = ready_creature(&mut state, P1, 4, 4);

    state.stack.push(StackEntry::Trigger(PendingTrigger::EnteredBattlefield {
        object_id: banshee,
        card_id: banshee_card,
        controller: P0,
        description: "Morkrut Banshee".into(),
        chosen_targets: vec![Target::Object(target_creature)],
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

    let snapcaster = named_creature(&mut state, &reg, "Snapcaster Mage", P0);
    let snapcaster_card = reg.get_id_by_name("Snapcaster Mage").unwrap();

    let instant = spell_in_hand(&mut state, &reg, "Think Twice", P0);
    state.move_object(instant, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger::EnteredBattlefield {
        object_id: snapcaster,
        card_id: snapcaster_card,
        controller: P0,
        description: "Snapcaster Mage".into(),
        chosen_targets: vec![Target::Object(instant)],
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

    let reaper = named_creature(&mut state, &reg, "Reaper from the Abyss", P0);
    let reaper_card = reg.get_id_by_name("Reaper from the Abyss").unwrap();

    let creature = ready_creature(&mut state, P1, 3, 3);

    state.stack.push(StackEntry::Trigger(PendingTrigger::EndStepTrigger {
        object_id: reaper,
        card_id: reaper_card,
        controller: P0,
        description: "Reaper from the Abyss".into(),
        chosen_targets: vec![Target::Object(creature)],
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

    let reaper = named_creature(&mut state, &reg, "Reaper from the Abyss", P0);
    let reaper_card = reg.get_id_by_name("Reaper from the Abyss").unwrap();

    let creature = ready_creature(&mut state, P1, 3, 3);

    state.stack.push(StackEntry::Trigger(PendingTrigger::EndStepTrigger {
        object_id: reaper,
        card_id: reaper_card,
        controller: P0,
        description: "Reaper from the Abyss".into(),
        chosen_targets: vec![Target::Object(creature)],
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

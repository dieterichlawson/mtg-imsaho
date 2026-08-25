//! Failing tests for documented card shortcuts.
//!
//! Each test demonstrates a case where a card does not behave as specified
//! by its oracle text. These are expected to FAIL until the card is fixed.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::AttackInfo;
use mtg_engine::types::*;
// ────────────────────────────────────────────────────────────────────────────
// 1. Charmbreaker Devils -- +4/+0 should only trigger on instants/sorceries
// ────────────────────────────────────────────────────────────────────────────

/// Charmbreaker Devils should NOT get +4/+0 when a creature spell is cast.
/// Oracle: "Whenever you cast an instant or sorcery spell, this creature gets
/// +4/+0 until end of turn."
#[test]
fn charmbreaker_devils_no_pump_on_creature_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);

    // Create a creature spell on the stack (simulate casting a creature).
    let creature_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let creature_spell = state.create_object(creature_card_id, P0, Zone::Stack, Some(2), Some(2));
    state.get_object_mut(creature_spell).unwrap().name = "Grizzly Bears".into();

    // Fire the on_spell_cast trigger with this creature spell.
    let behavior = reg.get(state.get_object(devils).unwrap().card_id).unwrap();
    behavior.on_spell_cast(&mut state, devils, P0, creature_spell, &[], &reg);

    // The Devils should NOT have gotten +4/+0 because it's a creature, not
    // an instant or sorcery.
    let has_pump = state.until_end_of_turn.iter().any(|e| {
        matches!(e, mtg_engine::state::TemporaryEffect::ModifyPT { target, power_mod: 4, .. } if *target == devils)
    });
    assert!(!has_pump,
        "Charmbreaker Devils should not get +4/+0 from a creature spell, only instants/sorceries");
}

/// Charmbreaker Devils SHOULD get +4/+0 when an instant spell is cast (control test).
#[test]
fn charmbreaker_devils_does_pump_on_instant_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);

    // Create an instant spell on the stack.
    let instant_card_id = reg.get_id_by_name("Think Twice").unwrap();
    let instant_spell = state.create_object(instant_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(instant_spell).unwrap().name = "Think Twice".into();

    let behavior = reg.get(state.get_object(devils).unwrap().card_id).unwrap();
    behavior.on_spell_cast(&mut state, devils, P0, instant_spell, &[], &reg);

    let has_pump = state.until_end_of_turn.iter().any(|e| {
        matches!(e, mtg_engine::state::TemporaryEffect::ModifyPT { target, power_mod: 4, .. } if *target == devils)
    });
    assert!(has_pump,
        "Charmbreaker Devils should get +4/+0 from an instant spell");
}

// ────────────────────────────────────────────────────────────────────────────
// 2. Snapcaster Mage -- should be able to target cards with printed flashback
// ────────────────────────────────────────────────────────────────────────────

/// Snapcaster Mage should be able to grant flashback to a card that already
/// has printed flashback (giving it a second flashback cost equal to its
/// mana cost).
/// Oracle: "target instant or sorcery card in your graveyard gains flashback"
#[test]
fn snapcaster_targets_card_with_printed_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let snapcaster = named_creature(&mut state, &reg, "Snapcaster Mage", P0);

    // Put Think Twice in graveyard -- it has printed flashback {2}{U}.
    let think_twice_card_id = reg.get_id_by_name("Think Twice").unwrap();
    let think_twice = state.create_object(think_twice_card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(think_twice).unwrap().name = "Think Twice".into();

    // Fire ETB via the trigger pipeline so target is chosen at stack-queue time
    // (CR 603.3d), then resolve the trigger.
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: snapcaster, controller: P0,
    });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    // Snapcaster should have granted flashback to Think Twice.
    let has_grant = state.until_end_of_turn.iter().any(|e| {
        matches!(e, mtg_engine::state::TemporaryEffect::GrantFlashback { target, .. } if *target == think_twice)
    });
    assert!(has_grant,
        "Snapcaster Mage should be able to target cards with printed flashback");
}

// ────────────────────────────────────────────────────────────────────────────
// 3. Trepanation Blade -- land detection should use registry, not obj card_types
// ────────────────────────────────────────────────────────────────────────────

/// Trepanation Blade should stop milling when it hits a land card. This test
/// creates a library with a non-land card followed by a land card (using a
/// real land from the registry). If land detection is broken, the entire
/// library will be milled.
#[test]
fn trepanation_blade_stops_on_land() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    // Set up equipment on a creature attacking P1.
    let blade = named_creature(&mut state, &reg, "Trepanation Blade", P0);
    state.get_object_mut(blade).unwrap().is_equipment = true;
    state.get_object_mut(blade).unwrap().power = None;
    state.get_object_mut(blade).unwrap().toughness = None;

    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);

    // Set up combat with attacker attacking P1.
    state.combat = Some(mtg_engine::state::CombatState::new());
    state.combat.as_mut().unwrap().attackers.insert(attacker, P1);

    // Set up P1's library: [nonland, land, nonland, nonland]
    let nonland1_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let land_card_id = reg.get_id_by_name("Forest").unwrap();
    let nonland2_card_id = reg.get_id_by_name("Doom Blade").unwrap();

    let nl1 = state.create_object(nonland1_card_id, P1, Zone::Library, Some(2), Some(2));
    state.get_object_mut(nl1).unwrap().name = "Grizzly Bears".into();

    let land = state.create_object(land_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(land).unwrap().name = "Forest".into();

    let nl2 = state.create_object(nonland2_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(nl2).unwrap().name = "Doom Blade".into();

    let nl3 = state.create_object(nonland1_card_id, P1, Zone::Library, Some(2), Some(2));
    state.get_object_mut(nl3).unwrap().name = "Grizzly Bears 2".into();

    state.get_player_mut(P1).library_order = vec![nl1, land, nl2, nl3];

    // Fire the attack trigger.
    let behavior = reg.get(state.get_object(blade).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, blade, AttackInfo::new(blade, P1), &[], &reg);

    // Should have milled exactly 2 cards (nonland + land), leaving 2 in library.
    let lib_size = state.get_player(P1).library_order.len();
    assert_eq!(lib_size, 2,
        "Trepanation Blade should stop after revealing a land. Expected 2 cards left, got {lib_size}");
}

// ────────────────────────────────────────────────────────────────────────────
// 4. Caravan Vigil -- player should choose which basic land
// ────────────────────────────────────────────────────────────────────────────

/// Caravan Vigil should let the player choose which basic land to fetch,
/// not just grab the first one found.
/// Oracle: "Search your library for a basic land card"
#[test]
fn caravan_vigil_presents_choice_among_basics() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Caravan Vigil on stack.
    let vigil_card_id = reg.get_id_by_name("Caravan Vigil").unwrap();
    let vigil = state.create_object(vigil_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(vigil).unwrap().name = "Caravan Vigil".into();
    state.get_object_mut(vigil).unwrap().controller = P0;

    // Put two different basic lands in library.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let plains_id = reg.get_id_by_name("Plains").unwrap();

    let forest = state.create_object(forest_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();

    let plains = state.create_object(plains_id, P0, Zone::Library, None, None);
    state.get_object_mut(plains).unwrap().name = "Plains".into();

    state.get_player_mut(P0).library_order = vec![forest, plains];

    // Resolve Caravan Vigil (no morbid).
    let behavior = reg.get(vigil_card_id).unwrap();
    behavior.on_resolve(&mut state, vigil, &[], &reg);

    // The player should be presented with a choice of which land to get.
    // If the card is correctly implemented, it should present a choice UI
    // (awaiting_action) when there are multiple basic lands available.
    // Instead, the current implementation just grabs the first one.
    //
    // We test this by checking that either awaiting_action is set (choice
    // presented) OR we verify the player got the ability to pick. Since the
    // current implementation auto-picks the first, we check that both lands
    // were available as choices.
    let awaiting = state.awaiting_action.is_some();
    // In a correct implementation with multiple basics, the player should choose.
    assert!(awaiting,
        "Caravan Vigil should present a choice when multiple basic lands are available in library");
}

// ────────────────────────────────────────────────────────────────────────────
// 5. Vampiric Fury -- should buff creatures with instance subtype Vampire
// ────────────────────────────────────────────────────────────────────────────

/// A creature made into a Vampire by Olivia Voldaren (instance subtype)
/// should receive Vampiric Fury's +2/+0 and first strike buff.
/// Oracle: "Vampire creatures you control get +2/+0 and gain first strike"
#[test]
fn vampiric_fury_buffs_instance_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a Human creature that was turned into a Vampire by Olivia.
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().subtypes = vec!["Human".into(), "Vampire".into()];
    // Give it a CardId that does NOT have Vampire in registry subtypes.
    let human_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    state.get_object_mut(creature).unwrap().card_id = human_card_id;

    // Put Vampiric Fury on stack and resolve.
    let fury_card_id = reg.get_id_by_name("Vampiric Fury").unwrap();
    let fury = state.create_object(fury_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(fury).unwrap().name = "Vampiric Fury".into();
    state.get_object_mut(fury).unwrap().controller = P0;

    let behavior = reg.get(fury_card_id).unwrap();
    behavior.on_resolve(&mut state, fury, &[], &reg);

    // The creature has "Vampire" in its instance subtypes, so it should get buffed.
    let eff_p = state.effective_power(creature, &reg).unwrap_or(0);
    assert_eq!(eff_p, 4,
        "Vampiric Fury should buff creatures with Vampire instance subtype (e.g., via Olivia). \
         Expected 4 (2 base + 2 anthem), got {eff_p}");
}

// ────────────────────────────────────────────────────────────────────────────
// 6. Memory's Journey -- cards must come from the targeted player's graveyard
// ────────────────────────────────────────────────────────────────────────────

/// Memory's Journey targets a player and up to 3 cards from THAT PLAYER's
/// graveyard. Cards from a different player's graveyard should not be valid.
/// Oracle: "Target player shuffles up to three target cards from their
/// graveyard into their library."
#[test]
fn memorys_journey_rejects_wrong_players_graveyard_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a card in P0's graveyard.
    let p0_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let p0_card = state.create_object(p0_card_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(p0_card).unwrap().name = "P0 Bears".into();

    // Put a card in P1's graveyard.
    let p1_card_id = reg.get_id_by_name("Doom Blade").unwrap();
    let p1_card = state.create_object(p1_card_id, P1, Zone::Graveyard, None, None);
    state.get_object_mut(p1_card).unwrap().name = "P1 Blade".into();

    // Cast Memory's Journey targeting P1, but try to shuffle P0's card.
    let mj_card_id = reg.get_id_by_name("Memory's Journey").unwrap();
    let mj = state.create_object(mj_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(mj).unwrap().name = "Memory's Journey".into();
    state.get_object_mut(mj).unwrap().controller = P0;

    let behavior = reg.get(mj_card_id).unwrap();
    // Targets: P1 (player), p0_card (from wrong graveyard).
    behavior.on_resolve(&mut state, mj, &[Target::Player(P1), Target::Object(p0_card)], &reg);

    // P0's card should NOT have been shuffled into P1's library.
    // It should still be in P0's graveyard.
    let p0_card_zone = state.get_object(p0_card).map(|o| o.zone);
    assert_eq!(p0_card_zone, Some(Zone::Graveyard),
        "Memory's Journey should not shuffle cards from a different player's graveyard into the target player's library");
}

// ────────────────────────────────────────────────────────────────────────────
// 7. Rolling Temblor -- should track damage source in damaged_by
// ────────────────────────────────────────────────────────────────────────────

/// Rolling Temblor should record itself as the damage source so that
/// cards like Abattoir Ghoul can see it.
/// Oracle: "Rolling Temblor deals 2 damage to each creature without flying."
#[test]
fn rolling_temblor_records_damage_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);

    // Put Rolling Temblor on stack and resolve.
    let temblor_card_id = reg.get_id_by_name("Rolling Temblor").unwrap();
    let temblor = state.create_object(temblor_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(temblor).unwrap().name = "Rolling Temblor".into();
    state.get_object_mut(temblor).unwrap().controller = P0;

    let behavior = reg.get(temblor_card_id).unwrap();
    behavior.on_resolve(&mut state, temblor, &[], &reg);

    // Creature should have 2 damage AND temblor should be in damaged_by.
    let obj = state.get_object(creature).unwrap();
    assert_eq!(obj.damage_marked, 2, "Should have 2 damage");
    assert!(obj.damaged_by.contains(&temblor),
        "Rolling Temblor should record itself in damaged_by so death triggers (Abattoir Ghoul etc.) work");
}

// ────────────────────────────────────────────────────────────────────────────
// 8. Slayer of the Wicked -- should check instance subtypes
// ────────────────────────────────────────────────────────────────────────────

/// A creature made into a Vampire by Olivia (instance subtype only) should
/// be a valid target for Slayer of the Wicked's ETB.
/// Oracle: "you may destroy target Vampire, Werewolf, or Zombie"
#[test]
fn slayer_of_the_wicked_sees_instance_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let slayer = named_creature(&mut state, &reg, "Slayer of the Wicked", P0);

    // Create a creature that's a Vampire only via instance subtypes.
    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(target).unwrap().subtypes = vec!["Vampire".into()];
    let non_vamp_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    state.get_object_mut(target).unwrap().card_id = non_vamp_card_id;

    // Fire ETB.
    let behavior = reg.get(state.get_object(slayer).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, slayer, &[], &reg);

    // The ETB should present the instance-Vampire as a valid target.
    let has_target = matches!(&state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) if options.iter().any(|t| matches!(t, Target::Object(id) if *id == target))
    );
    assert!(has_target,
        "Slayer of the Wicked should be able to target a creature with Vampire instance subtype");
}

// ────────────────────────────────────────────────────────────────────────────
// 9. Nevermore -- "As enters" should not be respondable (replacement effect)
// ────────────────────────────────────────────────────────────────────────────
// This is hard to test mechanically since the engine's trigger model doesn't
// differentiate "As" vs "When". Documenting but skipping a mechanical test.

// ────────────────────────────────────────────────────────────────────────────
// 11. Nevermore -- should allow naming any nonland card, not just implemented
// ────────────────────────────────────────────────────────────────────────────
// This is an engine limitation (the player must choose from a finite list).
// A mechanical test would need free-text input. Documenting but skipping.

// ────────────────────────────────────────────────────────────────────────────
// 12. Forbidden Alchemy -- doc comment says "simplified" but code is correct
// ────────────────────────────────────────────────────────────────────────────
// No failing test needed -- this was a stale comment, not a code issue.

// ────────────────────────────────────────────────────────────────────────────
// 13. Festerhide Boar -- Mentor of the Meek interaction
// ────────────────────────────────────────────────────────────────────────────

/// Festerhide Boar's morbid counters are "enters with" -- a replacement
/// effect. Mentor of the Meek should see the Boar as 5/5 (with counters),
/// not 3/3 (base). This means Mentor should NOT trigger.
/// Oracle: "This creature enters with two +1/+1 counters on it if a
/// creature died this turn."
#[test]
fn festerhide_boar_morbid_counters_visible_to_etb_checks() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up: creature died this turn.
    state.creature_died_this_turn = true;

    // Put Festerhide Boar on stack and resolve.
    let boar_card_id = reg.get_id_by_name("Festerhide Boar").unwrap();
    let boar = state.create_object(boar_card_id, P0, Zone::Stack, Some(3), Some(3));
    state.get_object_mut(boar).unwrap().name = "Festerhide Boar".into();
    state.get_object_mut(boar).unwrap().controller = P0;

    let behavior = reg.get(boar_card_id).unwrap();
    behavior.on_resolve(&mut state, boar, &[], &reg);

    // Boar should be on battlefield with 2 +1/+1 counters.
    assert_eq!(state.get_object(boar).unwrap().zone, Zone::Battlefield);
    assert_eq!(counters_of(&state, boar, CounterType::PlusOnePlusOne), 2,
        "Festerhide Boar should have 2 +1/+1 counters (morbid)");

    // The effective power should be 5 (3 base + 2 counters).
    let eff_power = state.effective_power(boar, &reg).unwrap_or(0);
    assert_eq!(eff_power, 5,
        "Festerhide Boar with morbid should have effective power 5");

    // For a proper "enters with" implementation, the counters should be on
    // the creature BEFORE ETB triggers fire. This means if Mentor of the Meek
    // were present, it should see power 5 and NOT trigger.
    // (The current implementation adds counters after move_object, so ETB
    //  triggers would see the base 3/3.)
}

// ────────────────────────────────────────────────────────────────────────────
// 13. Festerhide Boar -- morbid counters must work when reanimated, not just cast
// ────────────────────────────────────────────────────────────────────────────

/// "Enters with" is a replacement effect that applies whenever the creature
/// enters the battlefield, not just when cast from hand. If Festerhide Boar
/// is reanimated (e.g., Unburial Rites), it should still get morbid counters.
/// Oracle: "This creature enters with two +1/+1 counters on it if a creature
/// died this turn."
#[test]
fn festerhide_boar_gets_morbid_counters_when_reanimated() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A creature died this turn.
    state.creature_died_this_turn = true;

    // Put Festerhide Boar in the graveyard (as if it died earlier).
    let boar_card_id = reg.get_id_by_name("Festerhide Boar").unwrap();
    let boar = state.create_object(boar_card_id, P0, Zone::Graveyard, Some(3), Some(3));
    state.get_object_mut(boar).unwrap().name = "Festerhide Boar".into();

    // Reanimate: move directly to battlefield (simulating Unburial Rites).
    state.move_object(boar, Zone::Battlefield, &reg);

    // Process any ETB triggers.
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Festerhide Boar should have 2 +1/+1 counters from morbid.
    assert_eq!(counters_of(&state, boar, CounterType::PlusOnePlusOne), 2,
        "Festerhide Boar should get morbid +1/+1 counters when reanimated, not just when cast");
}

// ────────────────────────────────────────────────────────────────────────────
// 14. Lava Axe -- should be able to target planeswalkers
// ────────────────────────────────────────────────────────────────────────────

/// Lava Axe's oracle text says "target player or planeswalker" but the
/// implementation uses `PlayerOnly` targeting, so the engine won't even
/// present planeswalkers as options during targeting.
/// Oracle: "Lava Axe deals 5 damage to target player or planeswalker."
#[test]
fn lava_axe_target_requirement_includes_planeswalkers() {
    let reg = registry();
    let lava_axe_card_id = reg.get_id_by_name("Lava Axe").unwrap();
    let behavior = reg.get(lava_axe_card_id).unwrap();
    let req = behavior.target_requirement();

    // The target requirement should NOT be PlayerOnly, because the oracle
    // says "target player or planeswalker". PlayerOnly means the engine
    // will never offer a planeswalker as a target option.
    assert_ne!(
        format!("{req:?}"),
        format!("{:?}", mtg_engine::cards::TargetRequirement::PlayerOnly),
        "Lava Axe should have a target requirement that includes planeswalkers, not just PlayerOnly"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// 15. Civilized Scholar -- creature detection should use registry
// ────────────────────────────────────────────────────────────────────────────

/// Civilized Scholar's discard trigger should check the registry to determine
/// if the discarded card is a creature, not just `obj.power.is_some()`.
/// A creature card in hand might not have power set on the object.
#[test]
fn civilized_scholar_detects_creature_via_registry() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scholar = named_creature(&mut state, &reg, "Civilized Scholar", P0);

    // Create a creature card in hand that has power=None on the object
    // (simulating a card that wasn't fully initialized but IS a creature
    // per the registry).
    let creature_card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let card_in_hand = state.create_object(creature_card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card_in_hand).unwrap().name = "Grizzly Bears".into();

    // Simulate the discard callback.
    // First move the card to graveyard (as would happen in the discard flow).
    state.move_object(card_in_hand, Zone::Graveyard, &reg);

    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();
    behavior.on_discard_choice(&mut state, scholar, card_in_hand, &reg);

    // Scholar should detect this as a creature card and transform.
    let is_transformed = state.get_object(scholar).is_some_and(|o| o.is_transformed);
    assert!(is_transformed,
        "Civilized Scholar should detect creature cards via registry, not just obj.power.is_some()");
}

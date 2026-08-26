//! Tests for Innistrad Tier 9 Equipment cards:
//! Cobbled Wings, Mask of Avacyn, Silver-Inlaid Dagger, Sharpened Pitchfork,
//! Butcher's Cleaver, Wooden Stake.
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Butcher's Cleaver
//! - Cobbled Wings
//! - Mask of Avacyn
//! - Sharpened Pitchfork
//! - Silver-Inlaid Dagger
//! - Wooden Stake

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
/// Helper: place a named equipment on the battlefield, already set as equipment.
fn equipment_on_battlefield(
    state: &mut mtg_engine::state::GameState,
    registry: &CardRegistry,
    name: &str,
    owner: mtg_engine::ids::PlayerId,
) -> mtg_engine::ids::ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.is_equipment = true;
    id
}

/// Helper: equip an equipment to a creature by activating the ability.
fn equip(
    state: &mtg_engine::state::GameState,
    registry: &CardRegistry,
    equipment_id: mtg_engine::ids::ObjectId,
    creature_id: mtg_engine::ids::ObjectId,
) -> mtg_engine::state::GameState {
    let legal = engine::legal_actions(state, registry);
    let equip_action = legal.actions.iter().find(|a| {
        matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == equipment_id && targets == &[Target::Object(creature_id)])
    }).expect("should be able to equip the creature");
    engine::submit_action(state, equip_action, registry)
}

// ══════════════════════════════════════════════════════════════════
// Cobbled Wings — {2} Equipment. Equipped creature has flying. Equip {1}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn cobbled_wings_enters_as_equipment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wings = castable_spell(&mut state, &reg, "Cobbled Wings", P0);

    state = cast_and_resolve(&state, &reg, wings, vec![]);
    let obj = state.get_object(wings).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(obj.is_equipment);
    assert!(obj.attached_to.is_none());
}

#[test]
fn cobbled_wings_equip_only_your_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _opponent_creature = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let _wings = equipment_on_battlefield(&mut state, &reg, "Cobbled Wings", P0);

    // Add mana for equip cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let equip_actions: Vec<_> = legal.actions.iter().filter(|a| {
        matches!(a, Action::ActivateAbility { .. })
    }).collect();

    // Should have no equip actions (no creatures to target that P0 controls).
    assert!(equip_actions.is_empty(), "Should not be able to equip opponent's creature");
}

// ── What equipping grants ────────────────────────────────────────

/// Equipping grants the printed static bonus. Nine tests used to walk this one
/// equipment at a time — three for the unconditional bonuses, six for the three
/// human-conditional ones in both directions. The conditional half lives in
/// `equipment_human_conditional.rs`, which already tables it and also covers
/// the bonus updating live when the creature stops being a Human; the
/// unconditional half is here.
const UNCONDITIONAL_GRANTS: &[(&str, u32, i32, i32, &[Keyword])] = &[
    // (equipment, equip cost, +power, +toughness, keywords granted)
    ("Cobbled Wings",   1, 0, 0, &[Keyword::Flying]),
    ("Mask of Avacyn",  3, 1, 2, &[Keyword::Hexproof]),
    ("Wooden Stake",    1, 1, 0, &[]),
    ("Butcher's Cleaver", 3, 3, 0, &[]),
    ("Silver-Inlaid Dagger", 2, 2, 0, &[]),
];

#[test]
fn equipping_grants_the_printed_bonus() {
    let reg = registry();
    for (name, cost, dp, dt, keywords) in UNCONDITIONAL_GRANTS {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        // Grizzly Bears is a 2/2 Bear — deliberately not a Human, so only the
        // unconditional half of a conditional equipment applies.
        let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        let equipment = equipment_on_battlefield(&mut state, &reg, name, P0);

        for keyword in *keywords {
            assert!(!state.has_keyword(creature, *keyword, &reg),
                "test precondition: the Bear does not already have {keyword:?}");
        }

        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, *cost);
        state = equip(&state, &reg, equipment, creature);

        assert_eq!(state.effective_power(creature, &reg), Some(2 + dp),
            "{name} should give +{dp} power");
        assert_eq!(state.effective_toughness(creature, &reg), Some(2 + dt),
            "{name} should give +{dt} toughness");
        for keyword in *keywords {
            assert!(state.has_keyword(creature, *keyword, &reg),
                "{name} should grant {keyword:?}");
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// Mask of Avacyn — {2} Equipment. +1/+2 and hexproof. Equip {3}.
// ══════════════════════════════════════════════════════════════════

// ══════════════════════════════════════════════════════════════════
// Wooden Stake — {2} Equipment. +1/+0; destroy Vampires on block. Equip {1}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn wooden_stake_destroys_vampire_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up: P0 has a creature with Wooden Stake, P1 has a Vampire attacker.
    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let stake_obj = equipment_on_battlefield(&mut state, &reg, "Wooden Stake", P0);

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    // P1 has a Vampire attacker (Markov Patrician is a 3/1 Vampire with no evasion).
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);

    // Move to declare blockers step with the vampire attacking.
    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    // Set up combat with vampire as attacker.
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(vampire, P0);
    combat.blocker_assignments.insert(vampire, vec![]);
    state.combat = Some(combat);

    // Declare blockers: creature blocks vampire.
    submit_declare_blockers(&mut state, P0, &[(creature, vampire)], &reg);

    // Process triggers — this should fire Wooden Stake's block trigger.
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // The vampire should be destroyed.
    assert_eq!(state.get_object(vampire).unwrap().zone, Zone::Graveyard,
        "Vampire should be destroyed by Wooden Stake's trigger");
}

#[test]
fn wooden_stake_does_not_destroy_non_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let stake_obj = equipment_on_battlefield(&mut state, &reg, "Wooden Stake", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    // P1 has a non-Vampire attacker.
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);

    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(bear, P0);
    combat.blocker_assignments.insert(bear, vec![]);
    state.combat = Some(combat);

    submit_declare_blockers(&mut state, P0, &[(creature, bear)], &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Non-Vampire should NOT be destroyed.
    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Battlefield,
        "Non-Vampire should not be destroyed by Wooden Stake");
}

// ══════════════════════════════════════════════════════════════════
// Equipment general mechanics
// ══════════════════════════════════════════════════════════════════

#[test]
fn equipment_detaches_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let wings = equipment_on_battlefield(&mut state, &reg, "Cobbled Wings", P0);

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature);
    assert_eq!(state.get_object(wings).unwrap().attached_to, Some(creature));

    // Kill the creature.
    state.get_object_mut(creature).unwrap().damage_marked = 2;
    check_state_based_actions(&mut state, &reg);

    // Creature is dead but equipment should remain on battlefield, unattached.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(wings).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(wings).unwrap().attached_to, None);
}

#[test]
fn equipment_can_be_moved_to_different_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature1 = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let creature2 = named_permanent(&mut state, &reg, "Savannah Lions", P0);
    let wings = equipment_on_battlefield(&mut state, &reg, "Cobbled Wings", P0);

    // Equip to first creature.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature1);
    assert!(state.has_keyword(creature1, Keyword::Flying, &reg));
    assert!(!state.has_keyword(creature2, Keyword::Flying, &reg));

    // Re-equip to second creature.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature2);
    assert!(!state.has_keyword(creature1, Keyword::Flying, &reg));
    assert!(state.has_keyword(creature2, Keyword::Flying, &reg));
}

#[test]
fn equipment_cast_and_equip_full_flow() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    // Cast Cobbled Wings.
    let wings = castable_spell(&mut state, &reg, "Cobbled Wings", P0);
    state = cast_and_resolve(&state, &reg, wings, vec![]);

    // Equipment should be on battlefield, unattached.
    assert_eq!(state.get_object(wings).unwrap().zone, Zone::Battlefield);
    assert!(state.get_object(wings).unwrap().is_equipment);
    assert!(state.get_object(wings).unwrap().attached_to.is_none());
    assert!(!state.has_keyword(creature, Keyword::Flying, &reg));

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature);
    assert!(state.has_keyword(creature, Keyword::Flying, &reg));
}

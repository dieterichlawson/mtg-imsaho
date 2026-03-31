//! Tests for Innistrad Tier 9 Equipment cards:
//! Cobbled Wings, Mask of Avacyn, Silver-Inlaid Dagger, Sharpened Pitchfork,
//! Butcher's Cleaver, Wooden Stake.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Helper: place a named equipment on the battlefield, already set as equipment.
fn equipment_on_battlefield(
    state: &mut mtg_engine::state::GameState,
    registry: &CardRegistry,
    name: &str,
    owner: mtg_engine::ids::PlayerId,
) -> mtg_engine::ids::ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {}", name));
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
fn cobbled_wings_has_correct_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Cobbled Wings").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.name, "Cobbled Wings");
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
    assert!(data.power.is_none());
    assert!(data.toughness.is_none());
}

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
fn cobbled_wings_grants_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0);
    let wings = equipment_on_battlefield(&mut state, &reg, "Cobbled Wings", P0);

    // Creature should not have flying initially.
    assert!(!state.has_keyword(creature, Keyword::Flying, &reg));

    // Add mana for equip cost {1} and equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature);

    // Creature should now have flying.
    assert!(state.has_keyword(creature, Keyword::Flying, &reg));
}

#[test]
fn cobbled_wings_equip_only_your_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _opponent_creature = named_creature(&mut state, &reg, "Grizzly Bears", P1);
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

// ══════════════════════════════════════════════════════════════════
// Mask of Avacyn — {2} Equipment. +1/+2 and hexproof. Equip {3}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn mask_of_avacyn_has_correct_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Mask of Avacyn").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.name, "Mask of Avacyn");
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn mask_of_avacyn_grants_pt_and_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let mask = equipment_on_battlefield(&mut state, &reg, "Mask of Avacyn", P0);

    // Add mana for equip cost {3}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state = equip(&state, &reg, mask, creature);

    // Should be 3/4 with hexproof.
    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));
    assert!(state.has_keyword(creature, Keyword::Hexproof, &reg));
}

// ══════════════════════════════════════════════════════════════════
// Silver-Inlaid Dagger — {1} Equipment. +2/+0, or +3/+0 if Human. Equip {2}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn silver_inlaid_dagger_has_correct_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Silver-Inlaid Dagger").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.name, "Silver-Inlaid Dagger");
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 1);
}

#[test]
fn silver_inlaid_dagger_non_human_gets_plus_2() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2, Bear
    let dagger = equipment_on_battlefield(&mut state, &reg, "Silver-Inlaid Dagger", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state = equip(&state, &reg, dagger, creature);

    // Non-Human: +2/+0 -> 4/2.
    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
}

#[test]
fn silver_inlaid_dagger_human_gets_plus_3() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Champion of the Parish is a Human.
    let human = named_creature(&mut state, &reg, "Champion of the Parish", P0); // 1/1 Human
    let dagger = equipment_on_battlefield(&mut state, &reg, "Silver-Inlaid Dagger", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state = equip(&state, &reg, dagger, human);

    // Human: +3/+0 -> 4/1.
    assert_eq!(state.effective_power(human, &reg), Some(4));
    assert_eq!(state.effective_toughness(human, &reg), Some(1));
}

// ══════════════════════════════════════════════════════════════════
// Sharpened Pitchfork — {2} Equipment. First strike; +1/+1 if Human. Equip {1}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn sharpened_pitchfork_has_correct_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Sharpened Pitchfork").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.name, "Sharpened Pitchfork");
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn sharpened_pitchfork_non_human_gets_first_strike_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2, Bear
    let fork = equipment_on_battlefield(&mut state, &reg, "Sharpened Pitchfork", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, fork, creature);

    // Non-Human: first strike but no P/T bonus.
    assert!(state.has_keyword(creature, Keyword::FirstStrike, &reg));
    assert_eq!(state.effective_power(creature, &reg), Some(2));
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
}

#[test]
fn sharpened_pitchfork_human_gets_first_strike_and_bonus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let human = named_creature(&mut state, &reg, "Champion of the Parish", P0); // 1/1 Human
    let fork = equipment_on_battlefield(&mut state, &reg, "Sharpened Pitchfork", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, fork, human);

    // Human: first strike + +1/+1 -> 2/2.
    assert!(state.has_keyword(human, Keyword::FirstStrike, &reg));
    assert_eq!(state.effective_power(human, &reg), Some(2));
    assert_eq!(state.effective_toughness(human, &reg), Some(2));
}

// ══════════════════════════════════════════════════════════════════
// Butcher's Cleaver — {3} Equipment. +3/+0; lifelink if Human. Equip {3}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn butchers_cleaver_has_correct_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Butcher's Cleaver").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.name, "Butcher's Cleaver");
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 3);
}

#[test]
fn butchers_cleaver_non_human_gets_power_no_lifelink() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2, Bear
    let cleaver = equipment_on_battlefield(&mut state, &reg, "Butcher's Cleaver", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state = equip(&state, &reg, cleaver, creature);

    // Non-Human: +3/+0 -> 5/2, no lifelink.
    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
    assert!(!state.has_keyword(creature, Keyword::Lifelink, &reg));
}

#[test]
fn butchers_cleaver_human_gets_power_and_lifelink() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let human = named_creature(&mut state, &reg, "Champion of the Parish", P0); // 1/1 Human
    let cleaver = equipment_on_battlefield(&mut state, &reg, "Butcher's Cleaver", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    state = equip(&state, &reg, cleaver, human);

    // Human: +3/+0 -> 4/1, with lifelink.
    assert_eq!(state.effective_power(human, &reg), Some(4));
    assert_eq!(state.effective_toughness(human, &reg), Some(1));
    assert!(state.has_keyword(human, Keyword::Lifelink, &reg));
}

// ══════════════════════════════════════════════════════════════════
// Wooden Stake — {2} Equipment. +1/+0; destroy Vampires on block. Equip {1}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn wooden_stake_has_correct_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Wooden Stake").unwrap();
    let data = reg.card_data(id).unwrap();
    assert_eq!(data.name, "Wooden Stake");
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn wooden_stake_grants_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let stake = equipment_on_battlefield(&mut state, &reg, "Wooden Stake", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake, creature);

    // +1/+0 -> 3/2.
    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
}

#[test]
fn wooden_stake_destroys_vampire_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up: P0 has a creature with Wooden Stake, P1 has a Vampire attacker.
    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let stake = equipment_on_battlefield(&mut state, &reg, "Wooden Stake", P0);

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake, creature);

    // P1 has a Vampire attacker (Markov Patrician is a 3/1 Vampire with no evasion).
    let vampire = named_creature(&mut state, &reg, "Markov Patrician", P1);

    // Move to declare blockers step with the vampire attacking.
    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    // Set up combat with vampire as attacker.
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(vampire, P0);
    combat.blocker_assignments.insert(vampire, vec![]);
    state.combat = Some(combat);

    // Declare blockers: creature blocks vampire.
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(creature, vampire)], &reg);

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

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0);
    let stake = equipment_on_battlefield(&mut state, &reg, "Wooden Stake", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake, creature);

    // P1 has a non-Vampire attacker.
    let bear = named_creature(&mut state, &reg, "Grizzly Bears", P1);

    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(bear, P0);
    combat.blocker_assignments.insert(bear, vec![]);
    state.combat = Some(combat);

    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(creature, bear)], &reg);
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

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let wings = equipment_on_battlefield(&mut state, &reg, "Cobbled Wings", P0);

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature);
    assert_eq!(state.get_object(wings).unwrap().attached_to, Some(creature));

    // Kill the creature.
    state.get_object_mut(creature).unwrap().damage_marked = 2;
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Creature is dead but equipment should remain on battlefield, unattached.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(wings).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(wings).unwrap().attached_to, None);
}

#[test]
fn equipment_can_be_moved_to_different_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature1 = named_creature(&mut state, &reg, "Grizzly Bears", P0);
    let creature2 = named_creature(&mut state, &reg, "Savannah Lions", P0);
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

    let creature = named_creature(&mut state, &reg, "Grizzly Bears", P0);

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

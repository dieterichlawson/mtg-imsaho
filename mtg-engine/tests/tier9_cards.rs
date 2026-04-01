//! Tests for Innistrad Tier 9 cards: equipment and artifacts.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ══════════════════════════════════════════════════════════════════
// Traveler's Amulet
// ══════════════════════════════════════════════════════════════════

#[test]
fn travelers_amulet_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Traveler's Amulet").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(!data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 1);
}

#[test]
fn travelers_amulet_finds_basic_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put the amulet on the battlefield.
    let amulet = named_creature(&mut state, &reg, "Traveler's Amulet", P0);

    // Put a basic Forest in P0's library.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    {
        let obj = state.get_object_mut(forest).unwrap();
        obj.name = "Forest".into();
        obj.card_types = vec![CardType::Land];
    }
    state.get_player_mut(P0).library_order.push(forest);

    // Add mana for the ability: {1}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: amulet,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // Amulet should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(amulet).unwrap().zone,
        Zone::Graveyard,
        "Traveler's Amulet should be sacrificed"
    );

    // Forest should be in hand.
    assert_eq!(
        new_state.get_object(forest).unwrap().zone,
        Zone::Hand,
        "Forest should have been moved to hand"
    );
}

// ══════════════════════════════════════════════════════════════════
// Demonmail Hauberk
// ══════════════════════════════════════════════════════════════════

#[test]
fn demonmail_hauberk_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Demonmail Hauberk").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 4);
}

#[test]
fn demonmail_hauberk_equip_sacrifices_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Demonmail Hauberk on the battlefield.
    let hauberk = named_equipment(&mut state, &reg, "Demonmail Hauberk", P0);

    // Two creatures: one to sacrifice, one to equip.
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 2, 2);

    // Equip costs sacrifice a creature (no mana cost).
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: hauberk,
            ability_index: 0,
            targets: vec![Target::Object(creature_b)],
        },
        &reg,
    );

    // Hauberk should be attached to creature_b.
    assert_eq!(
        new_state.get_object(hauberk).unwrap().attached_to,
        Some(creature_b),
        "Demonmail Hauberk should be attached to the target creature"
    );

    // Exactly one creature should have been sacrificed.
    let a_zone = new_state.get_object(creature_a).unwrap().zone;
    let b_zone = new_state.get_object(creature_b).unwrap().zone;
    let sacrificed_count = [a_zone, b_zone].iter()
        .filter(|z| **z == Zone::Graveyard)
        .count();
    assert_eq!(sacrificed_count, 1,
        "Exactly one creature should be sacrificed to equip (a={:?}, b={:?})", a_zone, b_zone);

    // If creature_b survived (wasn't the one sacrificed), it should have the bonus.
    if b_zone == Zone::Battlefield {
        assert_eq!(new_state.effective_power(creature_b, &reg), Some(6));
        assert_eq!(new_state.effective_toughness(creature_b, &reg), Some(4));
    }
}

// ══════════════════════════════════════════════════════════════════
// Runechanter's Pike
// ══════════════════════════════════════════════════════════════════

#[test]
fn runechanters_pike_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Runechanter's Pike").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn runechanters_pike_grants_first_strike_and_power_bonus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Runechanter's Pike on the battlefield and attach it.
    let pike = named_equipment(&mut state, &reg, "Runechanter's Pike", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Manually attach the pike.
    state.get_object_mut(pike).unwrap().attached_to = Some(creature);

    // No instants/sorceries in graveyard yet — creature should be 2/2 with first strike.
    assert_eq!(state.effective_power(creature, &reg), Some(2));
    assert!(state.has_keyword(creature, Keyword::FirstStrike, &reg),
        "Equipped creature should have first strike");

    // Put 2 instant cards and 1 sorcery card in the graveyard.
    let bolt_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bolt1 = state.create_object(bolt_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(bolt1).unwrap().card_types = vec![CardType::Instant];

    let bolt2 = state.create_object(bolt_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(bolt2).unwrap().card_types = vec![CardType::Instant];

    let div_id = reg.get_id_by_name("Divination").unwrap();
    let div = state.create_object(div_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(div).unwrap().card_types = vec![CardType::Sorcery];

    // Now creature should get +3/+0 (3 instant/sorcery cards).
    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
}

#[test]
fn runechanters_pike_equip_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pike = named_equipment(&mut state, &reg, "Runechanter's Pike", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Add mana for equip: {2}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: pike,
            ability_index: 0,
            targets: vec![Target::Object(creature)],
        },
        &reg,
    );

    assert_eq!(
        new_state.get_object(pike).unwrap().attached_to,
        Some(creature),
        "Pike should be attached to the creature"
    );
}

// ══════════════════════════════════════════════════════════════════
// Inquisitor's Flail
// ══════════════════════════════════════════════════════════════════

#[test]
fn inquisitors_flail_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Inquisitor's Flail").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn inquisitors_flail_doubles_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    // Attach the flail.
    state.get_object_mut(flail).unwrap().attached_to = Some(creature);

    // Creature's effective power should NOT be doubled (no more dynamic_pt hack).
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "3/3 creature with Inquisitor's Flail should still show 3 effective power");

    // Set up combat: creature attacks P1 unblocked.
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(creature, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    let life_before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    let life_after = state.get_player(P1).life;

    // 3 damage doubled = 6 damage.
    assert_eq!(life_before - life_after, 6,
        "Inquisitor's Flail should double combat damage to player");
}

#[test]
fn inquisitors_flail_equip_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let flail = named_equipment(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: flail,
            ability_index: 0,
            targets: vec![Target::Object(creature)],
        },
        &reg,
    );

    assert_eq!(
        new_state.get_object(flail).unwrap().attached_to,
        Some(creature),
        "Flail should be attached to the creature"
    );
}

// ══════════════════════════════════════════════════════════════════
// Trepanation Blade
// ══════════════════════════════════════════════════════════════════

#[test]
fn trepanation_blade_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Trepanation Blade").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 3);
}

#[test]
fn trepanation_blade_attack_trigger_mills_and_pumps() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Attach the blade.
    state.get_object_mut(blade).unwrap().attached_to = Some(creature);

    // Set up P1's library with: [nonland, nonland, land].
    // Use card IDs that have card_types set.
    let bolt_card_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();

    let lib1 = state.create_object(bolt_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(lib1).unwrap().card_types = vec![CardType::Instant];

    let lib2 = state.create_object(bolt_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(lib2).unwrap().card_types = vec![CardType::Instant];

    let lib3 = state.create_object(forest_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(lib3).unwrap().card_types = vec![CardType::Land];

    state.get_player_mut(P1).library_order = vec![lib1, lib2, lib3];

    // Declare creature as attacker.
    combat::declare_attackers(&mut state, &[(creature, P1)], &reg);

    // Process triggers — the blade's attack trigger should fire.
    triggers::process_triggers(&mut state, &reg);

    // All 3 cards should be in graveyard (2 nonlands + 1 land).
    assert_eq!(state.get_object(lib1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(lib2).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(lib3).unwrap().zone, Zone::Graveyard);

    // P1's library should be empty.
    assert!(state.get_player(P1).library_order.is_empty());

    // Creature should get +3/+0 until end of turn (3 cards milled).
    assert_eq!(state.effective_power(creature, &reg), Some(5),
        "2/2 creature should have +3/+0 from Trepanation Blade (3 cards milled)");
    assert_eq!(state.effective_toughness(creature, &reg), Some(2),
        "Toughness should be unchanged");
}

#[test]
fn trepanation_blade_stops_at_first_land() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(blade).unwrap().attached_to = Some(creature);

    // Library starts with a land card.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let lib1 = state.create_object(forest_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(lib1).unwrap().card_types = vec![CardType::Land];

    let bolt_card_id = reg.get_id_by_name("Lightning Bolt").unwrap();
    let lib2 = state.create_object(bolt_card_id, P1, Zone::Library, None, None);
    state.get_object_mut(lib2).unwrap().card_types = vec![CardType::Instant];

    state.get_player_mut(P1).library_order = vec![lib1, lib2];

    combat::declare_attackers(&mut state, &[(creature, P1)], &reg);
    triggers::process_triggers(&mut state, &reg);

    // Only the first card (land) should be milled.
    assert_eq!(state.get_object(lib1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(lib2).unwrap().zone, Zone::Library,
        "Second card should remain in library (stopped at first land)");

    // +1/+0 for the one card milled.
    assert_eq!(state.effective_power(creature, &reg), Some(3));
}

// ══════════════════════════════════════════════════════════════════
// Blazing Torch
// ══════════════════════════════════════════════════════════════════

#[test]
fn blazing_torch_card_data() {
    let reg = registry();
    let id = reg.get_id_by_name("Blazing Torch").unwrap();
    let data = reg.card_data(id).unwrap();
    assert!(data.card_types.contains(&CardType::Artifact));
    assert!(data.subtypes.contains(&"Equipment".to_string()));
    assert_eq!(data.cost.as_ref().unwrap().mana_value(), 1);
}

#[test]
fn blazing_torch_grants_damage_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    // The creature should have the "{T}, Sacrifice Blazing Torch: deal 2 damage" ability.
    let legal = engine::legal_actions(&state, &reg);
    let has_torch_ability = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, ability_index: 1, .. }
        if *object_id == creature
    ));
    assert!(has_torch_ability, "Creature should have Blazing Torch's damage ability");
}

#[test]
fn blazing_torch_deals_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: creature,
            ability_index: 1,
            targets: vec![Target::Player(P1)],
        },
        &reg,
    );

    // P1 should have taken 2 damage.
    assert_eq!(new_state.get_player(P1).life, 18,
        "Blazing Torch should deal 2 damage to target player");

    // Creature should be tapped (tap cost).
    assert!(new_state.get_object(creature).unwrap().tapped,
        "Creature should be tapped from activating the ability");

    // Torch should be in graveyard (sacrificed).
    assert_eq!(new_state.get_object(torch).unwrap().zone, Zone::Graveyard,
        "Blazing Torch should be sacrificed");
}

#[test]
fn blazing_torch_deals_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    let enemy = ready_creature(&mut state, P1, 3, 3);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: creature,
            ability_index: 1,
            targets: vec![Target::Object(enemy)],
        },
        &reg,
    );

    // Enemy should have 2 damage marked.
    assert_eq!(new_state.get_object(enemy).unwrap().damage_marked, 2,
        "Target creature should have 2 damage from Blazing Torch");
}

#[test]
fn blazing_torch_damage_source_is_torch_not_creature() {
    // Per ruling: "The source of the damage is Blazing Torch, not the equipped creature."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    let enemy = ready_creature(&mut state, P1, 3, 3);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: creature,
            ability_index: 1,
            targets: vec![Target::Object(enemy)],
        },
        &reg,
    );

    // The damage source tracked on the creature should be the torch, not the equipped creature.
    let enemy_obj = new_state.get_object(enemy).unwrap();
    assert!(enemy_obj.damaged_by.contains(&torch),
        "Damage source should be the torch ({}), not the creature ({}). Got: {:?}",
        torch, creature, enemy_obj.damaged_by);
    assert!(!enemy_obj.damaged_by.contains(&creature),
        "Damage source should NOT be the equipped creature");
}

#[test]
fn blazing_torch_equip_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: torch,
            ability_index: 0,
            targets: vec![Target::Object(creature)],
        },
        &reg,
    );

    assert_eq!(
        new_state.get_object(torch).unwrap().attached_to,
        Some(creature),
        "Torch should be attached to the creature"
    );
}

#[test]
fn blazing_torch_equip_only_own_creatures() {
    // Equip says "target creature you control" — can't equip opponent's creatures.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let own_creature = ready_creature(&mut state, P0, 2, 2);
    let opp_creature = ready_creature(&mut state, P1, 3, 3);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let actions = engine::legal_actions(&state, &reg);
    let equip_targets: Vec<_> = actions.actions.iter()
        .filter_map(|a| {
            if let Action::ActivateAbility { object_id, ability_index: 0, targets } = a {
                if *object_id == torch { Some(targets.clone()) } else { None }
            } else { None }
        })
        .collect();

    // Should be able to equip own creature.
    assert!(equip_targets.iter().any(|t| t.contains(&Target::Object(own_creature))),
        "Should be able to equip own creature");
    // Should NOT be able to equip opponent's creature.
    assert!(!equip_targets.iter().any(|t| t.contains(&Target::Object(opp_creature))),
        "Should NOT be able to equip opponent's creature");
}

// ══════════════════════════════════════════════════════════════════
// Equipment enters battlefield unattached
// ══════════════════════════════════════════════════════════════════

#[test]
fn equipment_enters_unattached() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cast Demonmail Hauberk.
    let hauberk = castable_spell(&mut state, &reg, "Demonmail Hauberk", P0);

    let new_state = cast_and_resolve(&state, &reg, hauberk, vec![]);

    // Hauberk should be on the battlefield, unattached, with is_equipment = true.
    let obj = new_state.get_object(hauberk).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(obj.is_equipment, "Hauberk should be equipment");
    assert!(obj.attached_to.is_none(), "Equipment should enter unattached");
}

// ══════════════════════════════════════════════════════════════════
// Equipment detaches when creature dies
// ══════════════════════════════════════════════════════════════════

#[test]
fn equipment_detaches_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pike = named_equipment(&mut state, &reg, "Runechanter's Pike", P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(pike).unwrap().attached_to = Some(creature);

    // Kill the creature by marking lethal damage.
    state.get_object_mut(creature).unwrap().damage_marked = 10;

    // Run SBAs to kill the creature.
    while check_state_based_actions_with_registry(&mut state, Some(&reg)) {}

    // Creature should be dead.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);

    // Pike should be detached but still on battlefield.
    let pike_obj = state.get_object(pike).unwrap();
    assert_eq!(pike_obj.zone, Zone::Battlefield, "Equipment stays on battlefield");
    assert!(pike_obj.attached_to.is_none(), "Equipment should be detached");
}

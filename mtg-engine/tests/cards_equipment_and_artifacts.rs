//! Equipment and other artifacts: equip costs, what they grant, and what happens
//! when the creature they are attached to leaves.
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Blazing Torch
//! - Demonmail Hauberk
//! - Inquisitor's Flail
//! - Runechanter's Pike
//! - Traveler's Amulet
//! - Trepanation Blade

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::triggers;
use mtg_engine::types::*;

// ══════════════════════════════════════════════════════════════════
// Traveler's Amulet
// ══════════════════════════════════════════════════════════════════

#[test]
fn travelers_amulet_finds_basic_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put the amulet on the battlefield.
    let amulet = named_permanent(&mut state, &reg, "Traveler's Amulet", P0);

    // Put a basic Forest in P0's library.
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    {
        let obj = state.get_object_mut(forest).unwrap();
        obj.name = "Forest".into();
    }
    state.get_player_mut(P0).library_order.push(forest);

    // Add mana for the ability: {1}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = activate(&state, &reg, amulet, 0, vec![]);

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
fn demonmail_hauberk_equip_sacrifices_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Demonmail Hauberk on the battlefield.
    let hauberk = named_equipment(&mut state, &reg, "Demonmail Hauberk", P0);

    // Two creatures: one to sacrifice (creature_a), one to equip (creature_b).
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 2, 2);

    // Equip costs sacrifice a creature (no mana cost). The player explicitly
    // chooses creature_a as the sacrifice and creature_b as the equip target.
    let new_state = activate_sacrificing(&state, &reg, hauberk, 0, vec![Target::Object(creature_b)], creature_a);

    // Hauberk should be attached to creature_b (the target, not the sacrifice).
    assert_eq!(
        new_state.get_object(hauberk).unwrap().attached_to,
        Some(creature_b),
        "Demonmail Hauberk should be attached to the equip target"
    );

    // creature_a should have been sacrificed.
    assert_eq!(
        new_state.get_object(creature_a).unwrap().zone,
        Zone::Graveyard,
        "creature_a should have been sacrificed to pay the equip cost"
    );
    // creature_b should still be on the battlefield with the +4/+2 bonus.
    assert_eq!(
        new_state.get_object(creature_b).unwrap().zone,
        Zone::Battlefield,
        "creature_b (the target) should still be on the battlefield"
    );
    assert_eq!(new_state.effective_power(creature_b, &reg), Some(6),
        "creature_b should be 2+4 = 6 power");
    assert_eq!(new_state.effective_toughness(creature_b, &reg), Some(4),
        "creature_b should be 2+2 = 4 toughness");
}

// ══════════════════════════════════════════════════════════════════
// Runechanter's Pike
// ══════════════════════════════════════════════════════════════════

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
    state.create_object(bolt_id, P0, Zone::Graveyard, None, None);
    state.create_object(bolt_id, P0, Zone::Graveyard, None, None);

    let div_id = reg.get_id_by_name("Divination").unwrap();
    state.create_object(div_id, P0, Zone::Graveyard, None, None);

    // Now creature should get +3/+0 (3 instant/sorcery cards).
    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
}

// ══════════════════════════════════════════════════════════════════
// Inquisitor's Flail
// ══════════════════════════════════════════════════════════════════

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
        ..Default::default()
    });

    let life_before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    let life_after = state.get_player(P1).life;

    // 3 damage doubled = 6 damage.
    assert_eq!(life_before - life_after, 6,
        "Inquisitor's Flail should double combat damage to player");
}

// ══════════════════════════════════════════════════════════════════
// Trepanation Blade
// ══════════════════════════════════════════════════════════════════

/// "Whenever equipped creature attacks, defending player reveals cards from the
/// top of their library until they reveal a land card. That creature gets +1/+0
/// until end of turn for each card revealed this way."
///
/// The land is revealed *and* milled, so it counts too — the pump is the number
/// of cards that left the library, not the number of nonlands before the land.
#[test]
fn trepanation_blade_reveals_through_the_first_land_and_counts_it() {
    // (library from the top, cards milled, resulting power of a 2/2)
    const CASES: &[(&[&str], usize, i32)] = &[
        (&["Lightning Bolt", "Lightning Bolt", "Forest"], 3, 5),
        (&["Forest", "Lightning Bolt"], 1, 3),
    ];

    for &(library, milled, power) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::DeclareAttackers, P0);

        let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
        let creature = ready_creature(&mut state, P0, 2, 2);
        state.get_object_mut(blade).unwrap().attached_to = Some(creature);

        let cards: Vec<ObjectId> = library.iter()
            .map(|n| {
                let id = reg.get_id_by_name(n).unwrap();
                state.create_object(id, P1, Zone::Library, None, None)
            })
            .collect();
        state.get_player_mut(P1).library_order = cards.clone();

        submit_declare_attackers(&mut state, &[(creature, P1)], &reg);
        triggers::process_triggers(&mut state, &reg);

        for (n, &id) in cards.iter().enumerate() {
            let expected = if n < milled { Zone::Graveyard } else { Zone::Library };
            assert_eq!(state.get_object(id).unwrap().zone, expected,
                "{library:?}: card {n} ({}) should be in {expected:?}", library[n]);
        }
        assert_eq!(state.effective_power(creature, &reg), Some(power),
            "{library:?}: +1/+0 for each of the {milled} cards revealed");
        assert_eq!(state.effective_toughness(creature, &reg), Some(2),
            "{library:?}: toughness is untouched");
    }
}

// ══════════════════════════════════════════════════════════════════
// Blazing Torch
// ══════════════════════════════════════════════════════════════════

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

    let new_state = activate(&state, &reg, creature, 1, vec![Target::Player(P1)]);

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

/// "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
///
/// Ruling: "The source of the damage is Blazing Torch, not the equipped
/// creature." That matters for protection and for damage-source watchers, and
/// it is the half a test asserting only the 2 damage would miss.
#[test]
fn blazing_torch_deals_its_damage_as_the_torch_not_the_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_equipment(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);
    let enemy = ready_creature(&mut state, P1, 3, 3);

    let new_state = activate(&state, &reg, creature, 1, vec![Target::Object(enemy)]);

    let enemy_obj = new_state.get_object(enemy).unwrap();
    assert_eq!(enemy_obj.damage_marked, 2, "the target creature takes 2");
    assert!(enemy_obj.damaged_by.contains(&torch),
        "and the source is the Torch ({torch}), not the creature ({creature}): {:?}",
        enemy_obj.damaged_by);
    assert!(!enemy_obj.damaged_by.contains(&creature),
        "the equipped creature did not deal this damage");
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
            if let Action::ActivateAbility { object_id, ability_index: 0, targets, .. } = a {
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

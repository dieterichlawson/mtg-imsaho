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
    // CR 701.19b: the search stops and asks, even with one candidate.
    let new_state = answer_library_search(&new_state, &reg, Some(forest));

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

/// The same rule through the shared search helper: a mandatory search of a
/// hidden zone may still come back with nothing (CR 701.19b), and the library
/// is shuffled either way (CR 701.19a). The helper used to take the only
/// matching card for the player.
#[test]
fn a_mandatory_library_search_may_still_find_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let amulet = named_permanent(&mut state, &reg, "Traveler's Amulet", P0);
    let forest_card_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(forest);
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..8 {
        let id = state.create_object(bears_id, P0, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P0).library_order.push(id);
    }
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let state = activate(&state, &reg, amulet, 0, vec![]);
    let before: Vec<_> = state.get_player(P0).library_order.clone();
    assert!(state.awaiting_action.is_some(),
        "the only basic land is offered, not taken");

    let state = answer_library_search(&state, &reg, None);

    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Library,
        "declining leaves the land in the library");
    assert_ne!(state.get_player(P0).library_order, before,
        "the search happened, so the library is shuffled anyway");
}

// ══════════════════════════════════════════════════════════════════
// Demonmail Hauberk
// ══════════════════════════════════════════════════════════════════

#[test]
fn demonmail_hauberk_equip_sacrifices_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Demonmail Hauberk on the battlefield.
    let hauberk = named_permanent(&mut state, &reg, "Demonmail Hauberk", P0);

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
    let pike = named_permanent(&mut state, &reg, "Runechanter's Pike", P0);
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

/// Ruling 2011-09-22: "The value of X is constantly updated as instant cards
/// and sorcery cards are put into or removed from your graveyard." The
/// existing test covers cards going in; this one covers them coming out, and
/// what does not count while they are there.
#[test]
fn runechanters_pike_counts_only_instants_and_sorceries_in_its_own_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pike = named_permanent(&mut state, &reg, "Runechanter's Pike", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(pike).unwrap().attached_to = Some(creature);

    let bolt = reg.get_id_by_name("Lightning Bolt").unwrap();
    let bears = reg.get_id_by_name("Grizzly Bears").unwrap();

    // A creature card in the graveyard is not an instant or a sorcery.
    state.create_object(bears, P0, Zone::Graveyard, None, None);
    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "a creature card in the graveyard adds nothing");

    // Nor is a token, whatever its types: "instant and sorcery *cards*"
    // (CR 109.1), and a token sits in the graveyard until the next
    // state-based action pass.
    let token = state.create_token_with_subtypes(
        "", P0, 0, 0, vec![], vec![CardType::Instant], vec![], vec![], &reg)[0];
    state.move_object(token, Zone::Graveyard, &reg);
    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "a token in the graveyard is not a card");

    // Nor do an opponent's instants: the count is of *your* graveyard, and
    // "you" is the Pike's controller.
    state.create_object(bolt, P1, Zone::Graveyard, None, None);
    state.create_object(bolt, P1, Zone::Graveyard, None, None);
    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "an opponent's graveyard is not yours");

    let mine = state.create_object(bolt, P0, Zone::Graveyard, None, None);
    assert_eq!(state.effective_power(creature, &reg), Some(3));

    // Taken back out again — flashback exiles, Ghost Quarter's search shuffles
    // — and X follows it down.
    state.move_object(mine, Zone::Exile, &reg);
    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "X is updated as cards leave the graveyard, not only as they arrive");
}

/// "your graveyard" is the *Pike's* controller's, and an Equipment does not
/// change controller when the creature it is attached to does (CR 301.5c —
/// equip targets a creature you control, but an existing attachment survives).
#[test]
fn runechanters_pike_counts_its_own_controllers_graveyard_after_the_creature_is_stolen() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pike = named_permanent(&mut state, &reg, "Runechanter's Pike", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(pike).unwrap().attached_to = Some(creature);

    let bolt = reg.get_id_by_name("Lightning Bolt").unwrap();
    state.create_object(bolt, P0, Zone::Graveyard, None, None);
    state.create_object(bolt, P0, Zone::Graveyard, None, None);
    assert_eq!(state.effective_power(creature, &reg), Some(4), "test setup: 2/2 plus two");

    // An opponent takes the creature. The Pike stays theirs.
    state.get_object_mut(creature).unwrap().controller = P1;
    assert_eq!(state.effective_power(creature, &reg), Some(4),
        "X still counts the Pike controller's graveyard, not the thief's");
}

/// CR 608.2b: equip does nothing if its target is no longer a creature you
/// control when the ability resolves.
///
/// The engine's re-check for a `CreatureWithFilter` requirement only re-runs
/// the filter — it accepts a target in the Stack zone and asks nothing about
/// creature-ness — so the check at the moment of attaching is the card's, and
/// now lives once in `helpers::resolve_equip`.
#[test]
fn equip_does_not_attach_to_a_creature_that_left_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pike = named_permanent(&mut state, &reg, "Runechanter's Pike", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);

    let mut state = mtg_engine::engine::submit_action(&state, &Action::ActivateAbility {
        object_id: pike, ability_index: 0, targets: vec![Target::Object(creature)],
        tap_plan: vec![], sacrifice: None, x_value: None, source_card_id: None,
    }, &reg);

    // The creature is killed with the equip ability still on the stack.
    state.move_object(creature, Zone::Graveyard, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(pike).unwrap().attached_to, None,
        "nothing to attach to, so the Pike stays where it is");
}

// ══════════════════════════════════════════════════════════════════
// Inquisitor's Flail
// ══════════════════════════════════════════════════════════════════

#[test]
fn inquisitors_flail_doubles_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    // Attach the flail.
    state.get_object_mut(flail).unwrap().attached_to = Some(creature);

    // Creature's effective power should NOT be doubled (no more dynamic_pt hack).
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "3/3 creature with Inquisitor's Flail should still show 3 effective power");

    // Set up combat: creature attacks P1 unblocked.
    attacks_unblocked(&mut state, creature, P1);

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

        let blade = named_permanent(&mut state, &reg, "Trepanation Blade", P0);
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

    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
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

    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
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

    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
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

    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
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

    // Hauberk should be on the battlefield and unattached.
    let obj = new_state.get_object(hauberk).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(state.is_equipment(obj.id, &reg), "Hauberk should be equipment");
    assert!(obj.attached_to.is_none(), "Equipment should enter unattached");
}

// -------------------------------------------------------------------------
// Inquisitor's Flail
// -------------------------------------------------------------------------

/// A creature with the Flail attached, ready to attack.
fn equipped(state: &mut GameState, reg: &mtg_engine::cards::CardRegistry,
            power: i32, toughness: i32) -> ObjectId {
    let creature = ready_creature(state, P0, power, toughness);
    let flail = named_permanent(state, reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(creature);
    creature
}

/// Damage the equipped creature *deals* in combat is doubled — to a player and
/// to a blocking creature alike — and is not doubled when the Flail is on the
/// battlefield but attached to nothing. The unattached row is the control:
/// "8 damage" alone is also what an engine that doubles unconditionally does.
#[test]
fn the_equipped_creature_deals_double_combat_damage() {
    let reg = registry();

    // Unblocked, to a player.
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let creature = equipped(&mut state, &reg, 4, 4);
    attacks_unblocked(&mut state, creature, P1);
    let before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(before - state.get_player(P1).life, 8,
        "a 4-power attacker with the Flail deals 8 to the player");

    // Blocked, to the blocker.
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = equipped(&mut state, &reg, 2, 2);
    let blocker = ready_creature(&mut state, P1, 5, 5);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 4,
        "a 2-power attacker with the Flail deals 4 to its blocker");

    // Control: a Flail on the battlefield but equipping nothing doubles nothing.
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);
    attacks_unblocked(&mut state, creature, P1);
    let before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(before - state.get_player(P1).life, 3,
        "an unattached Flail doubles nothing");
}

/// The second clause: combat damage dealt *to* the equipped creature by
/// another source is doubled too.
#[test]
fn the_equipped_creature_takes_double_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = equipped(&mut state, &reg, 3, 6);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 4,
        "a 2-power blocker deals 4 to the equipped creature, not 2");
}

/// CR 616.1: several doubling replacements each apply once, so two Flails
/// quadruple rather than triple.
#[test]
fn two_flails_quadruple_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    for _ in 0..2 {
        let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
        state.get_object_mut(flail).unwrap().attached_to = Some(creature);
    }
    attacks_unblocked(&mut state, creature, P1);

    let before = state.get_player(P1).life;
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(before - state.get_player(P1).life, 12,
        "3 power doubled twice is 12, not 9");
}

/// Both clauses say *combat* damage, and fight is not combat damage. Neither
/// the damage the equipped creature deals in a fight nor the damage it takes
/// there is doubled.
#[test]
fn fight_damage_is_not_combat_damage_and_is_not_doubled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = equipped(&mut state, &reg, 3, 9);
    let opponent = ready_creature(&mut state, P1, 5, 5);

    mtg_engine::combat::fight(&mut state, creature, opponent, &reg);

    assert_eq!(state.get_object(opponent).unwrap().damage_marked, 3,
        "the equipped creature's fight damage is its power, undoubled");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 5,
        "and the fight damage it takes is undoubled too");
}

/// Ruling: "The land card is counted when calculating the bonus, and it will be
/// put into the graveyard with the other revealed cards."
///
/// The existing `trepanation_blade_stops_on_land` checks how many cards left
/// the library but never the bonus, and it passes the Blade itself as the
/// attacker — so the buff, if any, would have landed on the Equipment. This
/// checks the number the ruling is about.
#[test]
fn trepanation_blades_bonus_counts_the_land_it_stopped_on() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let blade = named_permanent(&mut state, &reg, "Trepanation Blade", P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);
    attacks_unblocked(&mut state, attacker, P1);

    // P1's library: one nonland, then a land. Two cards revealed in total.
    let nonland = state.create_object(
        reg.get_id_by_name("Doom Blade").unwrap(), P1, Zone::Library, None, None);
    let land = state.create_object(
        reg.get_id_by_name("Forest").unwrap(), P1, Zone::Library, None, None);
    let spare = state.create_object(
        reg.get_id_by_name("Doom Blade").unwrap(), P1, Zone::Library, None, None);
    state.get_player_mut(P1).library_order = vec![nonland, land, spare];

    let behavior = reg.get(state.get_object(blade).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, blade,
        mtg_engine::cards::AttackInfo::new(attacker, P1), &[], &reg);

    assert_eq!(state.get_object(land).unwrap().zone, Zone::Graveyard,
        "the land goes to the graveyard with the rest");
    assert_eq!(state.get_object(spare).unwrap().zone, Zone::Library,
        "and nothing past it is revealed");
    assert_eq!(state.effective_power(attacker, &reg), Some(4),
        "2/2 plus one for the nonland and one for the land it stopped on");
}

/// An all-nonland library is milled out rather than looping forever, and every
/// card revealed counts.
#[test]
fn trepanation_blade_stops_at_an_empty_library() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let blade = named_permanent(&mut state, &reg, "Trepanation Blade", P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);
    attacks_unblocked(&mut state, attacker, P1);

    let cards: Vec<ObjectId> = (0..3)
        .map(|_| state.create_object(
            reg.get_id_by_name("Doom Blade").unwrap(), P1, Zone::Library, None, None))
        .collect();
    state.get_player_mut(P1).library_order = cards.clone();

    let behavior = reg.get(state.get_object(blade).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, blade,
        mtg_engine::cards::AttackInfo::new(attacker, P1), &[], &reg);

    assert!(state.get_player(P1).library_order.is_empty(), "the library is emptied");
    assert_eq!(state.effective_power(attacker, &reg), Some(5), "2/2 plus three revealed");
}

/// "Equipped creature can't be blocked by Vampires or Zombies." — the Torch's
/// other half, which had no test at all.
#[test]
fn blazing_torch_stops_vampires_and_zombies_from_blocking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
    state.get_object_mut(torch).unwrap().attached_to = Some(attacker);

    // Bloodcrazed Neonate is a Vampire, Walking Corpse a Zombie, Avacyn's
    // Pilgrim neither.
    for (name, may_block) in [
        ("Bloodcrazed Neonate", false),
        ("Walking Corpse", false),
        ("Avacyn's Pilgrim", true),
    ] {
        let blocker = named_permanent(&mut state, &reg, name, P1);
        assert_eq!(
            mtg_engine::combat::can_block_attacker(&state, blocker, attacker, &reg),
            may_block,
            "{name} blocking a creature equipped with Blazing Torch");
    }
}

/// Ruling: "The source of the damage is Blazing Torch, not the equipped
/// creature. However, the equipped creature's ability is what targets the
/// permanent or player. ... It could target a creature with protection from
/// artifacts, but all the damage would be prevented."
///
/// Both halves at once: the target is legal (targeting is the creature's, and
/// the creature is not an artifact) and the damage is prevented (the damage is
/// the Torch's, and the Torch is).
#[test]
fn blazing_torch_targets_as_the_creature_and_damages_as_the_torch() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(torch).unwrap().attached_to = Some(creature);

    let warded = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(warded).unwrap().instance_continuous_effects =
        Some(vec![ContinuousEffect::ProtectionFrom {
            filter: CreatureFilter::HasCardType(CardType::Artifact),
            scope: EffectScope::OnSelf,
        }]);

    // Targeting: the ability is the creature's, and the creature is no
    // artifact, so protection from artifacts does not stop it being targeted.
    assert!(mtg_engine::engine::can_be_targeted_by(&state, warded, P0, Some(creature), &reg),
        "protection from artifacts does not stop the equipped creature's \
         ability from targeting");

    // Damage: the source is the Torch, which is an artifact, so all of it is
    // prevented.
    let new_state = activate(&state, &reg, creature, 1, vec![Target::Object(warded)]);
    assert_eq!(new_state.get_object(warded).unwrap().damage_marked, 0,
        "the damage's source is the Torch, an artifact, so protection from \
         artifacts prevents all of it");
}

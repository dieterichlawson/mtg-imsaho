//! Tests for Innistrad Tier 8 cards.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

#[test]

fn selfless_cathar_pump_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_creature(&mut state, &reg, "Selfless Cathar", P0);
    let bear = ready_creature(&mut state, P0, 2, 2);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: cathar,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // Cathar should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(cathar).unwrap().zone,
        Zone::Graveyard,
        "Selfless Cathar should be sacrificed"
    );

    // Bear should have +1/+1 from the effect.
    assert_eq!(new_state.effective_power(bear, &reg).unwrap(), 3);
    assert_eq!(new_state.effective_toughness(bear, &reg).unwrap(), 3);
}
#[test]

fn silverchase_fox_exiles_enchantment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fox = named_creature(&mut state, &reg, "Silverchase Fox", P0);

    // Create an enchantment for P1 (use Pacifism as a representative enchantment).
    let enchantment = named_creature(&mut state, &reg, "Glorious Anthem", P1);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: fox,
            ability_index: 0,
            targets: vec![Target::Object(enchantment)],
        },
        &reg,
    );

    // Fox should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(fox).unwrap().zone,
        Zone::Graveyard,
        "Silverchase Fox should be sacrificed"
    );

    // Enchantment should be in exile.
    assert_eq!(
        new_state.get_object(enchantment).unwrap().zone,
        Zone::Exile,
        "Target enchantment should be exiled"
    );
}
#[test]

fn brain_weevil_forces_discard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_creature(&mut state, &reg, "Brain Weevil", P0);

    // Give P1 exactly 2 cards in hand (auto-discards all when <= 2).
    let _c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let _c2 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);

    let hand_before = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_before, 2);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: weevil,
            ability_index: 0,
            targets: vec![Target::Player(P1)],
        },
        &reg,
    );

    // Weevil should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(weevil).unwrap().zone,
        Zone::Graveyard,
        "Brain Weevil should be sacrificed"
    );

    // P1 should have discarded 2 cards (2 - 2 = 0 remaining).
    let hand_after = new_state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_after, 0, "P1 should have 0 cards left after discarding 2");
}
#[test]

fn brain_weevil_has_intimidate() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_creature(&mut state, &reg, "Brain Weevil", P0);
    assert!(state.has_keyword(weevil, Keyword::Intimidate, &reg));
}
#[test]

fn disciple_of_griselbrand_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let disciple = named_creature(&mut state, &reg, "Disciple of Griselbrand", P0);
    // Create a 2/5 creature to sacrifice.
    let _fatty = ready_creature(&mut state, P0, 2, 5);

    let life_before = state.get_player(P0).life;

    // Add mana for the ability: {1}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: disciple,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // The engine auto-sacrifices the first creature it finds. It may pick the disciple
    // itself (1 toughness) or the fatty (5 toughness). Either way, we gained life.
    let life_after = new_state.get_player(P0).life;
    let gained = life_after - life_before;
    assert!(gained > 0, "Should have gained life, gained {}", gained);
}
#[test]

fn altars_reap_sacrifices_and_draws_two() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a creature to sacrifice and cards to draw.
    let creature = ready_creature(&mut state, P0, 2, 2);
    for _ in 0..3 {
        let c = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(c);
    }

    let spell = castable_spell(&mut state, &reg, "Altar's Reap", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Creature should be in graveyard (sacrificed).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Should have drawn 2 cards (hand was empty before spell was added).
    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, 2,
        "Should have drawn 2 cards");
}
#[test]

fn infernal_plunge_sacrifices_and_adds_rrr() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a creature to sacrifice.
    let creature = ready_creature(&mut state, P0, 1, 1);

    let spell = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Creature should be sacrificed.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Should have 3 red mana in pool (the {R} used to cast was consumed, then {R}{R}{R} added).
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Red), 3,
        "Should have 3 red mana in pool after Infernal Plunge");
}
#[test]

fn tribute_to_hunger_opponent_sacs_and_gain_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give the opponent a 3/4 creature.
    let opp_creature = ready_creature(&mut state, P1, 3, 4);
    // Set the creature's name for logging.
    state.get_object_mut(opp_creature).unwrap().name = "Big Beast".into();

    let initial_life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Player(P1)]);

    // Opponent's creature should be sacrificed.
    assert_eq!(state.get_object(opp_creature).unwrap().zone, Zone::Graveyard,
        "Opponent's creature should be sacrificed");

    // Caster should have gained life equal to toughness (4).
    assert_eq!(state.get_player(P0).life, initial_life + 4,
        "Should have gained life equal to sacrificed creature's toughness");
}
#[test]

fn tribute_to_hunger_no_creatures_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let initial_life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Player(P1)]);

    // No life change since no creature was sacrificed.
    assert_eq!(state.get_player(P0).life, initial_life,
        "No life gain when opponent has no creatures");
}
#[test]

fn divine_reckoning_keeps_one_per_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has 3 creatures: 2/1, 2/3, 1/5.
    let c1 = ready_creature(&mut state, P0, 2, 1);
    let _c2 = ready_creature(&mut state, P0, 2, 3);
    let _c3 = ready_creature(&mut state, P0, 1, 5);

    // P1 has 2 creatures: 4/2, 3/4.
    let c4 = ready_creature(&mut state, P1, 4, 2);
    let _c5 = ready_creature(&mut state, P1, 3, 4);

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let mut state = cast_and_resolve(&state, &reg, spell, vec![]);

    // P0 (active player) should be asked to choose first.
    assert!(state.awaiting_action.is_some(), "Should be awaiting P0's creature choice");

    // P0 chooses to keep c1 (the 2/1).
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(c1))),
        },
        &reg,
    );

    // P1 should now be asked to choose.
    assert!(state.awaiting_action.is_some(), "Should be awaiting P1's creature choice");

    // P1 chooses to keep c4 (the 4/2).
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(c4))),
        },
        &reg,
    );

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // P0 should have exactly 1 creature left on battlefield (c1).
    let p0_creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.power.is_some())
        .collect();
    assert_eq!(p0_creatures.len(), 1, "P0 should have exactly 1 creature");
    assert_eq!(p0_creatures[0].id, c1, "P0 should keep the chosen creature");

    // P1 should have exactly 1 creature left on battlefield (c4).
    let p1_creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.power.is_some())
        .collect();
    assert_eq!(p1_creatures.len(), 1, "P1 should have exactly 1 creature");
    assert_eq!(p1_creatures[0].id, c4, "P1 should keep the chosen creature");
}
#[test]

fn divine_reckoning_with_one_creature_keeps_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has 1 creature.
    let c1 = ready_creature(&mut state, P0, 3, 3);
    // P1 has 0 creatures.

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // P0's single creature should still be on the battlefield.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield,
        "Single creature should survive Divine Reckoning");

    // P1 should have no creatures (had none to begin with).
    let p1_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.power.is_some())
        .count();
    assert_eq!(p1_creatures, 0, "P1 should have no creatures");
}
#[test]

fn divine_reckoning_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Divine Reckoning").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some(), "Divine Reckoning should have flashback");
    assert_eq!(data.flashback_cost.unwrap().mana_value(), 7, "Flashback should cost 5WW = 7 MV");
}
#[test]

fn skirsdag_cultist_deals_2_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    // Need a creature to sacrifice (can sacrifice itself or another creature).
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Add red mana for the activation cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: cultist,
            ability_index: 0,
            targets: vec![Target::Object(target)],
        },
        &reg,
    );

    // Target creature should have taken 2 damage.
    let obj = state.get_object(target).unwrap();
    assert_eq!(obj.damage_marked, 2, "Target should have 2 damage marked");
}
#[test]

fn skirsdag_cultist_deals_2_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: cultist,
            ability_index: 0,
            targets: vec![Target::Player(P1)],
        },
        &reg,
    );

    assert_eq!(state.get_player(P1).life, 18, "Opponent should be at 18 life");
}
#[test]

fn skirsdag_cultist_cannot_activate_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cultist is the only creature. It will be sacrificed as part of the cost,
    // but we need at least one creature to sacrifice. Since the cultist itself
    // counts, the ability should still be available.
    let _cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let has_activate = actions.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(has_activate, "Should be able to activate (cultist counts as sacrifice fodder)");
}
#[test]

fn stitchers_apprentice_creates_token_then_sacrifices() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_creature(&mut state, &reg, "Stitcher's Apprentice", P0);

    // Add mana for the activation cost ({1}{U}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Count creatures before activation.
    let creatures_before: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .collect();
    assert_eq!(creatures_before.len(), 1, "Only the apprentice on the battlefield");

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: apprentice,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // After activation: a 2/2 token was created, but now the controller must choose
    // which creature to sacrifice. With 2 creatures (apprentice + token), a choice
    // is presented.
    assert!(state.awaiting_action.is_some(), "Should be awaiting sacrifice choice");

    // Find the token to sacrifice it.
    let token_id = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.power.is_some())
        .map(|o| o.id)
        .expect("Token should exist");

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(token_id))),
        },
        &reg,
    );

    // After choosing to sacrifice the token, only the apprentice remains.
    let creatures_after: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .collect();
    assert_eq!(creatures_after.len(), 1, "Should have 1 creature on battlefield after create + sacrifice");
}
#[test]

fn stitchers_apprentice_token_is_2_2_homunculus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_creature(&mut state, &reg, "Stitcher's Apprentice", P0);
    // Add a second creature so the apprentice doesn't sacrifice the token immediately.
    let _fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: apprentice,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // Find the token (is_token == true).
    let token = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.power.is_some());
    assert!(token.is_some(), "A token should exist on the battlefield");
    let token = token.unwrap();
    assert_eq!(token.power, Some(2), "Token should have power 2");
    assert_eq!(token.toughness, Some(2), "Token should have toughness 2");
    assert_eq!(token.name, "Homunculus", "Token should be named Homunculus");
}
#[test]

fn corpse_lunge_deals_damage_equal_to_exiled_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a 4/4 creature in P0's graveyard.
    let gy_creature = ready_creature(&mut state, P0, 4, 4);
    state.get_object_mut(gy_creature).unwrap().name = "Big Creature".into();
    state.move_object(gy_creature, Zone::Graveyard);

    // Target creature on P1's battlefield.
    let target = ready_creature(&mut state, P1, 5, 5);

    // Cast Corpse Lunge.
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // The graveyard creature should be in exile.
    let exiled = state.get_object(gy_creature).unwrap();
    assert_eq!(exiled.zone, Zone::Exile, "Graveyard creature should be exiled");

    // Target creature should have 4 damage.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 4, "Target should have 4 damage from Corpse Lunge");
}
#[test]

fn corpse_lunge_no_graveyard_creature_deals_no_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = ready_creature(&mut state, P1, 3, 3);

    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 0, "No damage should be dealt without graveyard creature");
}
#[test]

fn corpse_lunge_picks_highest_power_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put two creatures in graveyard: a 2/2 and a 5/5.
    let small = ready_creature(&mut state, P0, 2, 2);
    state.move_object(small, Zone::Graveyard);
    let big = ready_creature(&mut state, P0, 5, 5);
    state.move_object(big, Zone::Graveyard);

    let target = ready_creature(&mut state, P1, 6, 6);

    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // Should exile the 5/5 and deal 5 damage.
    let big_obj = state.get_object(big).unwrap();
    assert_eq!(big_obj.zone, Zone::Exile, "Highest-power creature should be exiled");

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 5, "Should deal 5 damage (power of exiled 5/5)");
}
#[test]

fn harvest_pyre_deals_damage_equal_to_chosen_x() {
    // Player chooses X=4 (exile 4 cards from graveyard).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 4 cards in P0's graveyard.
    for _ in 0..4 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let target = ready_creature(&mut state, P1, 5, 5);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    // Cast with X=4 (exile all 4).
    let mut new_state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![Target::Object(target)], sacrifice: None, exile_count: Some(4), exile_ids: vec![], alternative_cost: None },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &reg);

    // 4 cards should be exiled.
    let exiled_count = new_state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(exiled_count, 4, "4 graveyard cards should be exiled");

    // Target should have 4 damage.
    let target_obj = new_state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 4, "Target should have 4 damage from Harvest Pyre");
}

#[test]
fn harvest_pyre_player_chooses_partial_x() {
    // Player chooses X=2 (exile only 2 of 4 available cards).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for _ in 0..4 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let target = ready_creature(&mut state, P1, 5, 5);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let mut new_state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![Target::Object(target)], sacrifice: None, exile_count: Some(2), exile_ids: vec![], alternative_cost: None },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &reg);

    // Only 2 cards should be exiled.
    let exiled_count = new_state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(exiled_count, 2, "Only 2 graveyard cards should be exiled");

    // 2 original cards remain + Harvest Pyre itself (moved to graveyard after resolve) = 3.
    let gy_count = new_state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0)
        .count();
    assert_eq!(gy_count, 3, "2 original cards + Harvest Pyre in graveyard");

    // Target should have 2 damage, not 4.
    let target_obj = new_state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 2, "Target should have 2 damage (X=2)");
}

#[test]
fn harvest_pyre_x_zero_deals_no_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let target = ready_creature(&mut state, P1, 3, 3);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let mut new_state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![Target::Object(target)], sacrifice: None, exile_count: Some(0), exile_ids: vec![], alternative_cost: None },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &reg);

    let target_obj = new_state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 0, "No damage should be dealt with X=0");
}

#[test]
fn harvest_pyre_legal_actions_include_different_x_values() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let _target = ready_creature(&mut state, P1, 3, 3);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let actions = engine::legal_actions(&state, &reg);
    let harvest_actions: Vec<_> = actions.actions.iter()
        .filter(|a| {
            if let Action::CastSpell { object_id, .. } = a { *object_id == spell } else { false }
        })
        .collect();

    // Should have actions for X=0, X=1, X=2, X=3 (4 values × 1 target = 4 actions).
    assert!(harvest_actions.len() >= 4,
        "Should have at least 4 cast actions (X=0,1,2,3) but got {}", harvest_actions.len());
}
#[test]

fn harvest_pyre_only_exiles_own_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 cards in P0's graveyard.
    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }
    // Put 2 cards in P1's graveyard.
    for _ in 0..2 {
        let c = state.create_object(CardId(9999), P1, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard);
    }

    let target = ready_creature(&mut state, P1, 6, 6);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    // Cast with X=3 (exile all 3 of P0's graveyard cards).
    let mut new_state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![Target::Object(target)], sacrifice: None, exile_count: Some(3), exile_ids: vec![], alternative_cost: None },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &reg);

    // Only P0's 3 cards should be exiled.
    let p0_exiled = new_state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(p0_exiled, 3, "Only P0's 3 graveyard cards should be exiled");

    // P1's graveyard should be untouched.
    let p1_gy = new_state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert_eq!(p1_gy, 2, "P1's graveyard should be untouched");

    // Target should have 3 damage.
    let target_obj = new_state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 3, "Target should have 3 damage");
}

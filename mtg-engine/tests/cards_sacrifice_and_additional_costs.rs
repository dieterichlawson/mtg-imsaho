//! Spells whose cost is more than mana (CR 601.2b) — sacrifice a creature, exile
//! from a graveyard, discard — and the cards that make a player do the same.
//!
//! Cards covered (12), so this is greppable by name as well as by rule:
//!
//! - Altar's Reap
//! - Brain Weevil
//! - Corpse Lunge
//! - Disciple of Griselbrand
//! - Divine Reckoning
//! - Harvest Pyre
//! - Infernal Plunge
//! - Selfless Cathar
//! - Silverchase Fox
//! - Skirsdag Cultist
//! - Stitcher's Apprentice
//! - Tribute to Hunger

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
#[test]
fn selfless_cathar_pump_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_permanent(&mut state, &reg, "Selfless Cathar", P0);
    let bear = ready_creature(&mut state, P0, 2, 2);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = activate(&state, &reg, cathar, 0, vec![]);

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

    let fox = named_permanent(&mut state, &reg, "Silverchase Fox", P0);

    // Create an enchantment for P1 (use Pacifism as a representative enchantment).
    let enchantment = named_permanent(&mut state, &reg, "Glorious Anthem", P1);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = activate(&state, &reg, fox, 0, vec![Target::Object(enchantment)]);

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

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);

    // Give P1 exactly 2 cards in hand (auto-discards all when <= 2).
    let _c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let _c2 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);

    let hand_before = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_before, 2);

    let new_state = activate(&state, &reg, weevil, 0, vec![Target::Player(P1)]);

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
fn disciple_of_griselbrand_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    // Create a 2/5 creature to sacrifice for max life.
    let fatty = ready_creature(&mut state, P0, 2, 5);

    let life_before = state.get_player(P0).life;

    // Add mana for the ability: {1}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Player explicitly chooses to sacrifice the fatty (5 toughness → 5 life).
    let new_state = activate_sacrificing(&state, &reg, disciple, 0, vec![], fatty);

    // Should gain exactly 5 life (the fatty's toughness).
    let life_after = new_state.get_player(P0).life;
    assert_eq!(life_after - life_before, 5,
        "Should have gained 5 life from sacrificing the 2/5 fatty");
    assert_eq!(new_state.get_object(fatty).unwrap().zone, Zone::Graveyard,
        "fatty should have been sacrificed");
    assert_eq!(new_state.get_object(disciple).unwrap().zone, Zone::Battlefield,
        "disciple should still be on the battlefield");
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
/// "Target opponent sacrifices a creature **of their choice**." The choice is
/// the opponent's, not the caster's, and with more than one creature it has to
/// actually be presented to them. The existing test gives the opponent one
/// creature, where there is nothing to choose and the prompt never appears.
#[test]
fn tribute_to_hunger_lets_the_opponent_pick_which_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let small = ready_creature(&mut state, P1, 1, 1);
    let big = ready_creature(&mut state, P1, 5, 5);
    let life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let mut state = cast_onto_stack(&state, &reg, spell, vec![Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, .. }) = &state.awaiting_action else {
        panic!("the opponent has two creatures, so they must be asked which to \
                sacrifice; got {:?}", state.awaiting_action);
    };
    assert_eq!(*player, P1, "the choice belongs to the opponent, not the caster");

    let options = pending_choice_options(&state);
    assert!(options.contains(&Target::Object(small)) && options.contains(&Target::Object(big)),
        "both of the opponent's creatures are choosable: {options:?}");

    // They keep the big one.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(small))),
    }, &reg);

    assert_eq!(state.get_object(small).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(big).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_player(P0).life, life + 1,
        "life equal to the toughness of the creature they actually chose");
}

/// Ruling 2024-11-08: "Use the sacrificed creature's toughness as it last
/// existed on the battlefield to determine how much life to gain."
///
/// So the number is its toughness *including* whatever was modifying it, read
/// before it leaves — not its printed toughness, and not zero because it is
/// already in the graveyard by the time the life is added.
#[test]
fn tribute_to_hunger_gains_life_for_the_toughness_it_last_had() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    state.add_counters(creature, CounterType::PlusOnePlusOne, 3);
    assert_eq!(state.effective_toughness(creature, &reg), Some(5), "test setup");
    let life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P0).life, life + 5,
        "5, the toughness it last had on the battlefield — not its printed 2");
}

/// A sacrifice is not a destruction, so indestructible does not stop it
/// (CR 701.17a), and the creature does not get to regenerate.
#[test]
fn tribute_to_hunger_takes_an_indestructible_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Indestructible);

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "sacrifice is not destruction; indestructible does not apply");
}

/// The creature is chosen, not targeted — only the opponent is a target — so
/// hexproof has nothing to say about it (CR 702.11b is about targeting).
#[test]
fn tribute_to_hunger_can_take_a_hexproof_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Hexproof);

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "the creature is chosen by its controller, never targeted");
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

    check_state_based_actions(&mut state, &reg);

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
fn skirsdag_cultist_deals_2_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    // Need a creature to sacrifice (player picks the fodder, not the cultist).
    let fodder = ready_creature(&mut state, P0, 1, 1);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Add red mana for the activation cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = activate_sacrificing(&state, &reg, cultist, 0, vec![Target::Object(target)], fodder);

    // Target creature should have taken 2 damage.
    let obj = state.get_object(target).unwrap();
    assert_eq!(obj.damage_marked, 2, "Target should have 2 damage marked");
    // Cultist should still be alive (we sacrificed the fodder).
    assert_eq!(state.get_object(cultist).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(fodder).unwrap().zone, Zone::Graveyard);
}
#[test]
fn skirsdag_cultist_deals_2_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    let fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = activate_sacrificing(&state, &reg, cultist, 0, vec![Target::Player(P1)], fodder);

    assert_eq!(state.get_player(P1).life, 18, "Opponent should be at 18 life");
    assert_eq!(state.get_object(fodder).unwrap().zone, Zone::Graveyard);
}
#[test]
fn skirsdag_cultist_cannot_activate_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cultist is the only creature. It will be sacrificed as part of the cost,
    // but we need at least one creature to sacrifice. Since the cultist itself
    // counts, the ability should still be available.
    let _cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let has_activate = actions.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(has_activate, "Should be able to activate (cultist counts as sacrifice fodder)");
}
#[test]
fn stitchers_apprentice_creates_token_then_sacrifices() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_permanent(&mut state, &reg, "Stitcher's Apprentice", P0);

    // Add mana for the activation cost ({1}{U}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Count creatures before activation.
    let creatures_before: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .collect();
    assert_eq!(creatures_before.len(), 1, "Only the apprentice on the battlefield");

    let state = activate(&state, &reg, apprentice, 0, vec![]);

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

    let apprentice = named_permanent(&mut state, &reg, "Stitcher's Apprentice", P0);
    // Add a second creature so the apprentice doesn't sacrifice the token immediately.
    let _fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let state = activate(&state, &reg, apprentice, 0, vec![]);

    // Find the token (is_token == true).
    let token = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.power.is_some());
    assert!(token.is_some(), "A token should exist on the battlefield");
    let token = token.unwrap();
    assert_eq!(token.power, Some(2), "Token should have power 2");
    assert_eq!(token.toughness, Some(2), "Token should have toughness 2");
    assert_eq!(token.name, "Homunculus Token",
        "CR 111.4: an unnamed token is its subtypes plus \"Token\"");
}
#[test]
fn corpse_lunge_deals_damage_equal_to_exiled_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a 4/4 creature in P0's graveyard.
    let gy_creature = ready_creature(&mut state, P0, 4, 4);
    state.get_object_mut(gy_creature).unwrap().name = "Big Creature".into();
    state.move_object(gy_creature, Zone::Graveyard, &reg);

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
    state.move_object(small, Zone::Graveyard, &reg);
    let big = ready_creature(&mut state, P0, 5, 5);
    state.move_object(big, Zone::Graveyard, &reg);

    let target = ready_creature(&mut state, P1, 6, 6);

    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // Should exile the 5/5 and deal 5 damage.
    let big_obj = state.get_object(big).unwrap();
    assert_eq!(big_obj.zone, Zone::Exile, "Highest-power creature should be exiled");

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 5, "Should deal 5 damage (power of exiled 5/5)");
}
/// Harvest Pyre: "Exile X cards from your graveyard: this deals X damage to
/// target creature." Four tests walked one X each; X is the whole card, so it
/// is a table.
#[test]
fn harvest_pyre_exiles_x_of_your_own_cards_and_deals_x() {
    // (cards in your graveyard, cards in the opponent's, X chosen)
    const CASES: &[(usize, usize, u32)] = &[
        (4, 0, 4),  // exile everything
        (4, 0, 2),  // exile some — the rest stay
        (3, 0, 0),  // X=0 is legal and does nothing
        (3, 2, 3),  // "your graveyard": the opponent's cards are not touched
    ];
    let reg = registry();
    for &(mine, theirs, x) in CASES {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        for _ in 0..mine {
            let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
            state.move_object(c, Zone::Graveyard, &reg);
        }
        for _ in 0..theirs {
            let c = state.create_object(CardId(9999), P1, Zone::Battlefield, Some(1), Some(1));
            state.move_object(c, Zone::Graveyard, &reg);
        }
        let target = ready_creature(&mut state, P1, 6, 6);

        let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
        let mut state = engine::submit_action(
            &state,
            &Action::CastSpell {
                object_id: spell,
                targets: vec![Target::Object(target)],
                sacrifice: None,
                exile_count: Some(x),
                exile_ids: vec![],
                alternative_cost: None,
                tap_plan: vec![],
            },
            &reg,
        );
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        let exiled = state.objects.values()
            .filter(|o| o.zone == Zone::Exile && o.owner == P0)
            .count();
        assert_eq!(exiled, x as usize, "X={x} should exile {x} of your own cards");

        // Your remaining cards, plus Harvest Pyre itself once it has resolved.
        let left = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == P0)
            .count();
        assert_eq!(left, mine - x as usize + 1,
            "X={x} out of {mine} should leave {} of yours, plus Harvest Pyre",
            mine - x as usize);

        let theirs_left = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
            .count();
        assert_eq!(theirs_left, theirs, "the opponent's graveyard is never touched");

        assert_eq!(state.get_object(target).unwrap().damage_marked, x,
            "X={x} should deal {x} damage");
    }
}

#[test]
fn harvest_pyre_legal_actions_emits_single_cast_per_target() {
    // With the ChooseExileFromGraveyard refactor, the engine emits ONE
    // CastSpell per target for Harvest Pyre (not 2^gy_size entries per
    // target — the old subset enumeration). The player picks which
    // cards to exile via the resolution prompt.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
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

    // One target × one cast action = 1 (no subset enumeration).
    assert_eq!(harvest_actions.len(), 1,
        "Should have exactly 1 cast action (one per target; exile choice \
         happens via the ChooseExileFromGraveyard prompt), but got {}",
        harvest_actions.len());
}
// ── Bug H: CastableSpell.exile_x_from_gy_max ─────────────────────────
//
// Regression tests for BUG_REPORT_8SEAT.md Bug H. The LLM player was
// always casting Harvest Pyre with exile_count: None (→ X=0) because
// `choose_cast_targets` constructed a fresh CastSpell action instead
// of looking up one of the engine's pre-enumerated expanded variants,
// and the action label gave no hint at the effective X. The fix plumbs
// `exile_x_from_gy_max` through CastableSpell so the LLM UI can both
// display the damage in the label and find the matching expanded
// action. These tests pin the engine side of that contract.

#[test]
fn castable_spell_exposes_exile_x_from_gy_max_for_harvest_pyre() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put 5 cards in P0's graveyard. exile_x_from_gy_max should report 5.
    for _ in 0..5 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }
    // P1 graveyard cards must NOT count — exile_count is "your graveyard".
    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P1, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let _target = ready_creature(&mut state, P1, 6, 6);
    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Harvest Pyre should be castable");

    assert_eq!(cs.exile_x_from_gy_max, Some(5),
        "exile_x_from_gy_max should equal the caster's own graveyard size");
}

#[test]
fn castable_spell_exile_x_from_gy_max_is_none_for_non_exile_x_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let _target = ready_creature(&mut state, P1, 2, 2);
    let spell = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Lightning Bolt should be castable");

    assert!(cs.exile_x_from_gy_max.is_none(),
        "Spells without ExileXFromGraveyard must report None, got {:?}",
        cs.exile_x_from_gy_max);
}

#[test]
fn castable_spell_exile_x_from_gy_max_reports_zero_when_graveyard_empty() {
    // Empty graveyard → max X is zero, NOT None. Distinguishes "no exile
    // cost" from "exile cost with nothing to pay". The LLM label then
    // renders "X=0 (0 damage)" so the model doesn't waste the card.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let _target = ready_creature(&mut state, P1, 2, 2);
    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Harvest Pyre is castable even with an empty graveyard");

    assert_eq!(cs.exile_x_from_gy_max, Some(0),
        "empty graveyard must surface as Some(0), not None");
}

#[test]
fn harvest_pyre_max_x_cast_deals_full_damage_via_exile_prompt() {
    // Integration check for the new flow: cast Harvest Pyre, then
    // resolve the ChooseExileFromGraveyard prompt by exiling every
    // graveyard card (max X). Damage should equal graveyard size.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let target = ready_creature(&mut state, P1, 4, 4);
    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Harvest Pyre castable");
    let max_x = cs.exile_x_from_gy_max.expect("max X should be Some(3)");
    assert_eq!(max_x, 3);

    // cast_and_resolve picks the max-power (= max count for Harvest Pyre)
    // exile subset, then resolves the top of the stack.
    let new_state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    let target_obj = new_state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, max_x,
        "exiling max_x cards should deal max_x damage (={max_x})");
}

// -------------------------------------------------------------------------
// Infernal Plunge
// -------------------------------------------------------------------------

/// Cannot cast Infernal Plunge without a creature to sacrifice.
#[test]
fn cannot_cast_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if {
            state.get_object(*object_id)
                .is_some_and(|o| o.name == "Infernal Plunge")
        })
    });

    assert!(!can_cast,
        "Should not be able to cast Infernal Plunge without a creature to sacrifice");
}

/// Can cast Infernal Plunge when controlling a creature.
#[test]
fn can_cast_with_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let _creature = ready_creature(&mut state, P0, 1, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if {
            state.get_object(*object_id)
                .is_some_and(|o| o.name == "Infernal Plunge")
        })
    });

    assert!(can_cast,
        "Should be able to cast Infernal Plunge when controlling a creature");
}

/// Sacrifice happens at cast time (creature is gone before resolution).
#[test]
fn sacrifice_at_cast_time() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast with explicit sacrifice target.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge,
            targets: vec![],
            sacrifice: Some(creature),
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Creature should already be in graveyard (sacrificed at cast time).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature should be sacrificed at cast time, not resolution");

    // Spell should be on the stack.
    assert_eq!(state.get_object(plunge).unwrap().zone, Zone::Stack,
        "Infernal Plunge should be on the stack");
}

/// On resolution, adds {R}{R}{R} to mana pool.
#[test]
fn adds_three_red_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast with explicit sacrifice target.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge,
            targets: vec![],
            sacrifice: Some(creature),
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Record mana before resolution (should be 0 since we spent {R} to cast).
    let red_before = state.get_player(P0).mana_pool.get(ManaType::Red);

    // Resolve the spell.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let red_after = state.get_player(P0).mana_pool.get(ManaType::Red);
    assert_eq!(red_after - red_before, 3,
        "Infernal Plunge should add RRR on resolution");
}

/// Legal actions show one `CastSpell` per eligible creature to sacrifice.
#[test]
fn one_action_per_sacrifice_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 3, 3);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let plunge_actions: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, sacrifice, .. } = a {
            state.get_object(*object_id)
                .is_some_and(|o| o.name == "Infernal Plunge")
            && sacrifice.is_some()
        } else {
            false
        }
    }).collect();

    assert_eq!(plunge_actions.len(), 2,
        "Should have one CastSpell action per eligible creature (got {})", plunge_actions.len());

    // Both creatures should be represented as sacrifice options.
    let sac_ids: Vec<_> = plunge_actions.iter().filter_map(|a| {
        if let Action::CastSpell { sacrifice, .. } = a {
            *sacrifice
        } else {
            None
        }
    }).collect();
    assert!(sac_ids.contains(&creature_a), "Should include creature A as sacrifice option");
    assert!(sac_ids.contains(&creature_b), "Should include creature B as sacrifice option");
}

/// The creature list a player picks from is offered in a stable order.
///
/// It used to be built straight off `state.objects.values()`, a HashMap
/// iterator, so the order varied between runs of the same game. The player
/// picks from this list by position, so an unstable order means the same
/// decisions replay differently.
#[test]
fn divine_reckonings_choice_list_is_in_a_stable_order() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Several creatures for P0, so there is a real list to order.
    let mut mine: Vec<ObjectId> = (0..5).map(|_| ready_creature(&mut state, P0, 2, 2)).collect();
    mine.sort_by_key(|id| id.0);
    // And one for the opponent, which must not appear in P0's list.
    let theirs = ready_creature(&mut state, P1, 2, 2);

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    let options = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, .. })
            => options.clone(),
        _ => panic!("expected P0 to be choosing a creature to keep"),
    };

    let offered: Vec<ObjectId> = options.iter()
        .filter_map(|t| match t { Target::Object(id) => Some(*id), _ => None })
        .collect();

    assert_eq!(offered, mine, "offered in ascending object-id order");
    assert!(!offered.contains(&theirs), "and only creatures this player controls");
}

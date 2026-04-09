//! Tests for Innistrad Tier 6 cards.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Abattoir Ghoul ────────────────────────────────────────────────

/// Abattoir Ghoul gains life when a creature it damaged this turn dies.
#[test]
fn abattoir_ghoul_gains_life_from_damaged_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_creature(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3); // 2/3 creature

    // Simulate the ghoul having dealt damage to the victim this turn.
    state.get_object_mut(victim).unwrap().damaged_by.push(ghoul);
    // Mark lethal damage on the victim (3 toughness, 3 damage).
    state.get_object_mut(victim).unwrap().damage_marked = 3;

    // SBA should kill the victim.
    check_state_based_actions(&mut state, &reg);
    // Process death triggers.
    triggers::process_triggers(&mut state, &reg);

    // Ghoul's controller (P0) should have gained 3 life (victim's toughness).
    assert_eq!(state.get_player(P0).life, 23, "should gain life equal to victim's toughness");
}

/// Abattoir Ghoul does NOT gain life if it didn't damage the dying creature.
#[test]
fn abattoir_ghoul_no_life_if_not_damaged_by_ghoul() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _ghoul = named_creature(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3);

    // Victim dies without being damaged by the ghoul.
    state.get_object_mut(victim).unwrap().damage_marked = 3;

    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 20, "should NOT gain life");
}

/// Abattoir Ghoul uses last-known toughness (including +1/+1 counters).
#[test]
fn abattoir_ghoul_uses_last_known_toughness_with_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_creature(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3); // base 2/3

    // Give the victim a +1/+1 counter (effective toughness = 4).
    state.add_counters(victim, CounterType::PlusOnePlusOne, 1);

    // Ghoul damaged it.
    state.get_object_mut(victim).unwrap().damaged_by.push(ghoul);
    // Mark lethal damage (4 toughness with counter, 4 damage).
    state.get_object_mut(victim).unwrap().damage_marked = 4;

    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    // Should gain 4 life (last-known toughness with counter), not 3 (base).
    assert_eq!(state.get_player(P0).life, 24, "should gain life = last-known toughness including counters");
}

// ── Champion of the Parish ────────────────────────────────────────

/// Champion of the Parish gets a +1/+1 counter when another Human enters.
#[test]
fn champion_of_the_parish_counter_on_human_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    // Another Human enters the battlefield under our control.
    let human = named_creature(&mut state, &reg, "Unruly Mob", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P0,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(champion).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 1, "Champion should get a +1/+1 counter");
}

/// Champion does NOT trigger on non-Human creatures entering.
#[test]
fn champion_of_the_parish_no_counter_on_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    // A non-Human creature enters.
    let vampire = named_creature(&mut state, &reg, "Rakish Heir", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: vampire,
        controller: P0,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(champion).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 0, "Champion should NOT trigger on non-Human");
}

/// Champion does NOT trigger on opponent's Human.
#[test]
fn champion_of_the_parish_no_counter_on_opponent_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    // Opponent's Human enters.
    let human = named_creature(&mut state, &reg, "Unruly Mob", P1);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P1,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(champion).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 0, "Champion should NOT trigger on opponent's Human");
}

// ── Stromkirk Noble ───────────────────────────────────────────────

/// Stromkirk Noble gets +1/+1 counter on combat damage to player.
#[test]
fn stromkirk_noble_counter_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let noble = named_creature(&mut state, &reg, "Stromkirk Noble", P0);

    // Simulate combat damage to player.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: noble,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 1,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(noble).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 1, "Stromkirk Noble should get a +1/+1 counter");
}

/// Stromkirk Noble can't be blocked by Humans.
#[test]
fn stromkirk_noble_cant_be_blocked_by_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let noble = named_creature(&mut state, &reg, "Stromkirk Noble", P0);

    // Create a Human blocker.
    let human = named_creature(&mut state, &reg, "Unruly Mob", P1);

    // Set up combat.
    state.combat = Some(mtg_engine::state::CombatState::new());
    state.combat.as_mut().unwrap().attackers.insert(noble, P1);

    // Human should not be able to block the Noble.
    let can_block = combat::can_block_attacker(&state, human, noble, &reg);
    assert!(!can_block, "Humans should not be able to block Stromkirk Noble");

    // Non-Human should be able to block.
    let non_human = ready_creature(&mut state, P1, 2, 2);
    let can_block_nh = combat::can_block_attacker(&state, non_human, noble, &reg);
    assert!(can_block_nh, "Non-Humans should be able to block Stromkirk Noble");
}

// ── Rakish Heir ───────────────────────────────────────────────────

/// Rakish Heir gives +1/+1 to itself when it deals combat damage.
#[test]
fn rakish_heir_self_counter_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let heir = named_creature(&mut state, &reg, "Rakish Heir", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: heir,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(heir).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 1, "Rakish Heir should get a +1/+1 counter on itself");
}

/// Rakish Heir gives +1/+1 to other Vampires you control when they deal combat damage.
#[test]
fn rakish_heir_counter_on_other_vampire_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let heir = named_creature(&mut state, &reg, "Rakish Heir", P0);
    let other_vamp = named_creature(&mut state, &reg, "Stromkirk Noble", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: other_vamp,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 1,
    });
    triggers::process_triggers(&mut state, &reg);

    let vamp_counters = *state.get_object(other_vamp).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    // Stromkirk Noble gets +1/+1 from its own trigger AND +1/+1 from Rakish Heir.
    assert!(vamp_counters >= 2, "Other Vampire should get counters from both itself and Rakish Heir, got {}", vamp_counters);
}

/// Rakish Heir does NOT give +1/+1 to non-Vampire creatures.
#[test]
fn rakish_heir_no_counter_on_non_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _heir = named_creature(&mut state, &reg, "Rakish Heir", P0); // watcher
    let non_vamp = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(non_vamp).unwrap().summoning_sick = false;

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: non_vamp,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 3,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(non_vamp).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 0, "Non-Vampire should NOT get a counter from Rakish Heir");
}

// ── Bloodcrazed Neonate ───────────────────────────────────────────

/// Bloodcrazed Neonate must attack each combat (ForceAttack).
#[test]
fn bloodcrazed_neonate_forced_to_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let neonate = named_creature(&mut state, &reg, "Bloodcrazed Neonate", P0);

    // Check that the neonate has ForceAttack via continuous effects.
    let has_force_attack = state.has_continuous_effect(neonate, &|e| {
        match e {
            ContinuousEffect::ForceAttack { scope } => Some(scope),
            _ => None,
        }
    }, &reg);
    assert!(has_force_attack, "Bloodcrazed Neonate should have ForceAttack");
}

// ── Sturmgeist ────────────────────────────────────────────────────

/// Sturmgeist's P/T equals cards in hand.
#[test]
fn sturmgeist_pt_equals_hand_size() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sturmgeist = named_creature(&mut state, &reg, "Sturmgeist", P0);

    // Give P0 some cards in hand.
    for _ in 0..4 {
        state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Hand, None, None);
    }

    let power = state.effective_power(sturmgeist, &reg).unwrap();
    let toughness = state.effective_toughness(sturmgeist, &reg).unwrap();
    assert_eq!(power, 4, "Sturmgeist power should equal hand size");
    assert_eq!(toughness, 4, "Sturmgeist toughness should equal hand size");
}

// ── Falkenrath Marauders ──────────────────────────────────────────

/// Falkenrath Marauders gets TWO +1/+1 counters on combat damage.
#[test]
fn falkenrath_marauders_two_counters_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let marauders = named_creature(&mut state, &reg, "Falkenrath Marauders", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: marauders,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(marauders).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 2, "Falkenrath Marauders should get TWO +1/+1 counters");
}

// ── Balefire Dragon ───────────────────────────────────────────────

/// Balefire Dragon deals combat damage amount to all opponent's creatures.
#[test]
fn balefire_dragon_sweeps_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let dragon = named_creature(&mut state, &reg, "Balefire Dragon", P0);
    let opp_creature1 = ready_creature(&mut state, P1, 3, 4);
    let opp_creature2 = ready_creature(&mut state, P1, 2, 2);
    let own_creature = ready_creature(&mut state, P0, 1, 1); // should NOT be damaged

    // Dragon deals 6 combat damage to P1.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: dragon,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 6,
    });
    triggers::process_triggers(&mut state, &reg);

    // Opponent's creatures should each have 6 damage.
    assert_eq!(state.get_object(opp_creature1).unwrap().damage_marked, 6);
    assert_eq!(state.get_object(opp_creature2).unwrap().damage_marked, 6);
    // Own creature should be untouched.
    assert_eq!(state.get_object(own_creature).unwrap().damage_marked, 0);
}

// ── Curiosity ─────────────────────────────────────────────────────

/// Curiosity draws a card when enchanted creature deals combat damage to opponent (player says yes).
#[test]
fn curiosity_draw_on_enchanted_creature_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Create a creature and attach Curiosity to it.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let curiosity_card_id = reg.get_id_by_name("Curiosity").unwrap();
    let curiosity = state.create_object(curiosity_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(curiosity).unwrap().name = "Curiosity".into();
    state.get_object_mut(curiosity).unwrap().attached_to = Some(creature);

    // Give P0 a card in library to draw.
    let lib_card = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    let hand_before = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();

    // Enchanted creature deals combat damage to opponent.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: creature,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a yes/no choice for "you may draw a card".
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses yes.
    state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(true),
        },
        &reg,
    );

    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, hand_before + 1, "Should have drawn 1 card from Curiosity");
}

/// Curiosity does NOT draw a card when the player declines.
#[test]
fn curiosity_decline_draw() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Create a creature and attach Curiosity to it.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let curiosity_card_id = reg.get_id_by_name("Curiosity").unwrap();
    let curiosity = state.create_object(curiosity_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(curiosity).unwrap().name = "Curiosity".into();
    state.get_object_mut(curiosity).unwrap().attached_to = Some(creature);

    // Give P0 a card in library to draw.
    let lib_card = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    let hand_before = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();

    // Enchanted creature deals combat damage to opponent.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: creature,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a yes/no choice.
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses no.
    state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(false),
        },
        &reg,
    );

    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, hand_before, "Should NOT have drawn a card when declining");
}

// ── Stromkirk Patrol ──────────────────────────────────────────────

/// Stromkirk Patrol gets +1/+1 counter on combat damage.
#[test]
fn stromkirk_patrol_counter_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let patrol = named_creature(&mut state, &reg, "Stromkirk Patrol", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: patrol,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 4,
    });
    triggers::process_triggers(&mut state, &reg);

    let counters = *state.get_object(patrol).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(counters, 1, "Stromkirk Patrol should get a +1/+1 counter");
}

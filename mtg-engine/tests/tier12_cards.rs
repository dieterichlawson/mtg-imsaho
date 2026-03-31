//! Tests for Innistrad Tier 12 miscellaneous cards.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::triggers;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Scourge of Geier Reach ──────────────────────────────────────

/// Scourge gets +1/+1 for each creature opponents control.
#[test]
fn scourge_of_geier_reach_scales_with_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scourge = named_creature(&mut state, &reg, "Scourge of Geier Reach", P0);

    // No opponent creatures: base 3/3.
    assert_eq!(state.effective_power(scourge, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(scourge, &reg).unwrap(), 3);

    // Add 2 opponent creatures.
    ready_creature(&mut state, P1, 1, 1);
    ready_creature(&mut state, P1, 2, 2);

    // Should be 5/5 (3 + 2 opponent creatures).
    assert_eq!(state.effective_power(scourge, &reg).unwrap(), 5);
    assert_eq!(state.effective_toughness(scourge, &reg).unwrap(), 5);
}

/// Scourge doesn't count own creatures.
#[test]
fn scourge_of_geier_reach_ignores_own_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scourge = named_creature(&mut state, &reg, "Scourge of Geier Reach", P0);

    // Add friendly creatures - shouldn't affect P/T.
    ready_creature(&mut state, P0, 1, 1);
    ready_creature(&mut state, P0, 2, 2);

    assert_eq!(state.effective_power(scourge, &reg).unwrap(), 3);
    assert_eq!(state.effective_toughness(scourge, &reg).unwrap(), 3);
}

// ── Army of the Damned ──────────────────────────────────────────

/// Army creates 13 tapped Zombie tokens.
#[test]
fn army_of_the_damned_creates_13_tapped_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spell = castable_spell(&mut state, &reg, "Army of the Damned", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Count tokens on battlefield.
    let zombies: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Zombie" && o.controller == P0)
        .collect();
    assert_eq!(zombies.len(), 13, "Should have 13 Zombie tokens");

    // All should be tapped.
    for z in &zombies {
        assert!(z.tapped, "Zombie tokens should enter tapped");
    }

    // All should be 2/2.
    for z in &zombies {
        assert_eq!(z.power, Some(2));
        assert_eq!(z.toughness, Some(2));
    }
}

// ── Night Revelers ──────────────────────────────────────────────

/// Night Revelers has haste when opponent controls a Human.
#[test]
fn night_revelers_has_haste_with_opponent_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let revelers = named_creature(&mut state, &reg, "Night Revelers", P0);

    // No opponent Humans: no haste.
    assert!(!state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should not have haste without opponent Human");

    // Add a Human creature to the opponent.
    let human = named_creature(&mut state, &reg, "Champion of the Parish", P1);

    // Now should have haste.
    assert!(state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should have haste when opponent controls a Human");

    // Remove the Human.
    state.move_object(human, Zone::Graveyard);
    assert!(!state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should lose haste when opponent no longer controls a Human");
}

// ── Elite Inquisitor ────────────────────────────────────────────

/// Elite Inquisitor has first strike and vigilance.
#[test]
fn elite_inquisitor_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let inquisitor = named_creature(&mut state, &reg, "Elite Inquisitor", P0);

    assert!(state.has_keyword(inquisitor, Keyword::FirstStrike, &reg));
    assert!(state.has_keyword(inquisitor, Keyword::Vigilance, &reg));
}

/// Elite Inquisitor has protection from Vampires, Werewolves, Zombies.
/// Combat damage from those subtypes is prevented.
#[test]
fn elite_inquisitor_protection_prevents_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P1);

    let inquisitor = named_creature(&mut state, &reg, "Elite Inquisitor", P0);

    // Create a Vampire attacker.
    let vampire = named_creature(&mut state, &reg, "Markov Patrician", P1);

    // Set up combat: vampire attacks, inquisitor blocks.
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(vampire, P0);
    combat.blocker_assignments.insert(vampire, vec![inquisitor]);
    state.combat = Some(combat);

    // Deal combat damage.
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // Elite Inquisitor should take no damage from the Vampire.
    let inq_obj = state.get_object(inquisitor).unwrap();
    assert_eq!(inq_obj.damage_marked, 0, "Elite Inquisitor should not take damage from Vampires (protection)");
}

/// Elite Inquisitor's protection prevents Zombies from blocking it.
#[test]
fn elite_inquisitor_cant_be_blocked_by_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let inquisitor = named_creature(&mut state, &reg, "Elite Inquisitor", P0);
    let zombie = named_creature(&mut state, &reg, "Diregraf Ghoul", P1);

    // Zombie should not be able to block Elite Inquisitor (protection from Zombies).
    assert!(!mtg_engine::combat::can_block_attacker(&state, zombie, inquisitor, &reg),
        "Zombie should not be able to block Elite Inquisitor (protection from Zombies)");
}

// ── Ashmouth Hound ──────────────────────────────────────────────

/// Ashmouth Hound deals 1 damage when it blocks.
#[test]
fn ashmouth_hound_deals_damage_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);

    let hound = named_creature(&mut state, &reg, "Ashmouth Hound", P0);
    let attacker = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(attacker).unwrap().name = "Enemy".into();

    // Set up combat.
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(attacker, P0);
    combat.blocker_assignments.insert(attacker, vec![hound]);
    state.combat = Some(combat);

    // Fire blockers declared event.
    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(hound, attacker)],
    });
    triggers::process_triggers(&mut state, &reg);

    // The attacker should have 1 damage from Ashmouth Hound's trigger.
    let att = state.get_object(attacker).unwrap();
    assert_eq!(att.damage_marked, 1, "Ashmouth Hound should deal 1 damage to the creature it blocks");
}

// ── Hamlet Captain ──────────────────────────────────────────────

/// Hamlet Captain gives other Humans +1/+1 when it attacks.
#[test]
fn hamlet_captain_buffs_humans_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let captain = named_creature(&mut state, &reg, "Hamlet Captain", P0);
    let human = named_creature(&mut state, &reg, "Champion of the Parish", P0);
    let non_human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(non_human).unwrap().name = "Bear".into();

    // Declare attackers event with Hamlet Captain attacking.
    state.events.push(GameEvent::AttackersDeclared {
        attackers: vec![(captain, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Champion of the Parish should have +1/+1 buff.
    let champion_power = state.effective_power(human, &reg).unwrap();
    assert_eq!(champion_power, 2, "Champion should be 2 power (1 base + 1 from Hamlet Captain)");

    // Non-Human should not be affected.
    let bear_power = state.effective_power(non_human, &reg).unwrap();
    assert_eq!(bear_power, 2, "Non-human should still be 2 power");

    // Hamlet Captain itself should not get the buff (it says "other").
    let captain_power = state.effective_power(captain, &reg).unwrap();
    assert_eq!(captain_power, 2, "Hamlet Captain should not buff itself");
}

/// Hamlet Captain gives other Humans +1/+1 when it blocks.
#[test]
fn hamlet_captain_buffs_humans_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);

    let captain = named_creature(&mut state, &reg, "Hamlet Captain", P0);
    let human = named_creature(&mut state, &reg, "Elite Inquisitor", P0);
    let attacker = ready_creature(&mut state, P1, 3, 3);

    // Declare blockers event.
    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(captain, attacker)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Elite Inquisitor should have +1/+1 buff.
    let inq_power = state.effective_power(human, &reg).unwrap();
    assert_eq!(inq_power, 3, "Elite Inquisitor should be 3 power (2 base + 1 from Hamlet Captain)");
}

// ── Spare from Evil ─────────────────────────────────────────────

/// Spare from Evil gives protection from non-Human creatures.
#[test]
fn spare_from_evil_grants_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];
    state.get_object_mut(human).unwrap().name = "Human Warrior".into();

    let spell = castable_spell(&mut state, &reg, "Spare from Evil", P0);
    let mut state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Create a non-Human attacker (Zombie).
    let zombie = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(zombie).unwrap().subtypes = vec!["Zombie".into()];
    state.get_object_mut(zombie).unwrap().name = "Zombie".into();

    // The Zombie should not be able to block our Human (protection from non-Humans).
    assert!(!mtg_engine::combat::can_block_attacker(&state, zombie, human, &reg),
        "Non-Human creature should not be able to block a creature with protection from non-Humans");

    // A Human attacker should still be able to block.
    let human_opp = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(human_opp).unwrap().subtypes = vec!["Human".into()];
    assert!(mtg_engine::combat::can_block_attacker(&state, human_opp, human, &reg),
        "Human creature should still be able to block (protection only from non-Humans)");
}

// ── Burning Vengeance ───────────────────────────────────────────

/// Burning Vengeance deals 2 damage when you cast a flashback spell.
#[test]
fn burning_vengeance_triggers_on_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_creature(&mut state, &reg, "Burning Vengeance", P0);

    // Create a flashback spell on the stack, marked as cast_with_flashback.
    let spell = state.create_object(
        reg.get_id_by_name("Think Twice").unwrap(),
        P0,
        Zone::Stack,
        None,
        None,
    );
    state.get_object_mut(spell).unwrap().cast_with_flashback = true;
    state.get_object_mut(spell).unwrap().name = "Think Twice".into();

    // Fire SpellCast event.
    state.events.push(GameEvent::SpellCast { player: P0, object: spell });
    triggers::process_triggers(&mut state, &reg);

    // Opponent should have lost 2 life.
    assert_eq!(state.get_player(P1).life, 18,
        "Burning Vengeance should deal 2 damage to opponent on flashback cast");
}

/// Burning Vengeance does not trigger on normal spell casts.
#[test]
fn burning_vengeance_ignores_non_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_creature(&mut state, &reg, "Burning Vengeance", P0);

    // Create a normal spell on the stack (not flashback).
    let spell = state.create_object(
        reg.get_id_by_name("Think Twice").unwrap(),
        P0,
        Zone::Stack,
        None,
        None,
    );
    state.get_object_mut(spell).unwrap().name = "Think Twice".into();
    // cast_with_flashback defaults to false.

    state.events.push(GameEvent::SpellCast { player: P0, object: spell });
    triggers::process_triggers(&mut state, &reg);

    // Opponent should not have lost life.
    assert_eq!(state.get_player(P1).life, 20,
        "Burning Vengeance should NOT trigger on normal spell casts");
}

//! Instants, sorceries and enchantments whose behaviour is particular to the
//! card rather than to a rule the engine implements generally.
//!
//! Cards covered (13), so this is greppable by name as well as by rule:
//!
//! - Angelic Overseer
//! - Army of the Damned
//! - Ashmouth Hound
//! - Blasphemous Act
//! - Burning Vengeance
//! - Cackling Counterpart
//! - Elite Inquisitor
//! - Hamlet Captain
//! - Night Revelers
//! - Scourge of Geier Reach
//! - Sever the Bloodline
//! - Spare from Evil
//! - Traitorous Blood

mod common;

use common::*;
use mtg_engine::events::GameEvent;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ── Scourge of Geier Reach ──────────────────────────────────────

/// "Scourge of Geier Reach gets +1/+1 for each creature your opponents
/// control" — a characteristic-defining count that has to be recomputed as the
/// board changes, and has to count the right half of the board.
#[test]
fn scourge_of_geier_reach_counts_only_opponents_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scourge = named_permanent(&mut state, &reg, "Scourge of Geier Reach", P0);
    let pt = |s: &mtg_engine::state::GameState| {
        (s.effective_power(scourge, &reg).unwrap(), s.effective_toughness(scourge, &reg).unwrap())
    };

    assert_eq!(pt(&state), (3, 3), "an empty board leaves it at its printed 3/3");

    ready_creature(&mut state, P0, 1, 1);
    ready_creature(&mut state, P0, 2, 2);
    assert_eq!(pt(&state), (3, 3), "its controller's own creatures are not counted");

    ready_creature(&mut state, P1, 1, 1);
    ready_creature(&mut state, P1, 2, 2);
    assert_eq!(pt(&state), (5, 5), "two creatures across the table make it a 5/5");
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
    assert_eq!(count_tokens_named_by(&state, "Zombie", P0), 13, "Should have 13 Zombie tokens");

    for z in state.objects.values().filter(|o| o.is_token && o.name == "Zombie" && o.controller == P0) {
        assert!(z.tapped, "Zombie tokens should enter tapped");
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

    let revelers = named_permanent(&mut state, &reg, "Night Revelers", P0);

    // No opponent Humans: no haste.
    assert!(!state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should not have haste without opponent Human");

    // Add a Human creature to the opponent.
    let human = named_permanent(&mut state, &reg, "Champion of the Parish", P1);

    // Now should have haste.
    assert!(state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should have haste when opponent controls a Human");

    // Remove the Human.
    state.move_object(human, Zone::Graveyard, &reg);
    assert!(!state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should lose haste when opponent no longer controls a Human");
}

// ── Elite Inquisitor ────────────────────────────────────────────

/// Elite Inquisitor has protection from Vampires, Werewolves, Zombies.
/// Combat damage from those subtypes is prevented.
#[test]
fn elite_inquisitor_protection_prevents_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P1);

    let inquisitor = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);

    // Create a Vampire attacker.
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);

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

    let inquisitor = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    let zombie = named_permanent(&mut state, &reg, "Diregraf Ghoul", P1);

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

    let hound = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
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

    let captain = named_permanent(&mut state, &reg, "Hamlet Captain", P0);
    let human = named_permanent(&mut state, &reg, "Champion of the Parish", P0);
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

    let captain = named_permanent(&mut state, &reg, "Hamlet Captain", P0);
    let human = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
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

    let _bv = named_permanent(&mut state, &reg, "Burning Vengeance", P0);

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

    // Fire SpellCast event. CR 603.3d: "deals 2 damage to any target" needs a
    // target chosen as the trigger goes on the stack, so processing runs
    // through the helper that answers that prompt via `submit_action`, the way
    // a player would.
    state.events.push(GameEvent::SpellCast { player: P0, object: spell });
    process_triggers_auto_target_opponent(&mut state, &reg);
    // Opponent should have lost 2 life.
    assert_eq!(state.get_player(P1).life, 18,
        "Burning Vengeance should deal 2 damage to opponent on flashback cast");
}

/// Burning Vengeance does not trigger on normal spell casts.
#[test]
fn burning_vengeance_ignores_non_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_permanent(&mut state, &reg, "Burning Vengeance", P0);

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

// ── Traitorous Blood ───────────────────────────────────────────

/// Traitorous Blood steals a creature, untaps it, and grants haste + trample.
#[test]
fn traitorous_blood_steals_untaps_and_grants_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a tapped creature controlled by opponent.
    let enemy = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(enemy).unwrap().tapped = true;
    state.get_object_mut(enemy).unwrap().name = "Enemy Beast".into();

    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(enemy)]);

    // Creature should now be controlled by P0.
    let obj = state.get_object(enemy).unwrap();
    assert_eq!(obj.controller, P0, "Traitorous Blood should change controller to caster");
    assert!(!obj.tapped, "Traitorous Blood should untap the creature");

    // Should have haste and trample.
    assert!(state.has_keyword(enemy, Keyword::Haste, &reg),
        "Traitorous Blood should grant haste");
    assert!(state.has_keyword(enemy, Keyword::Trample, &reg),
        "Traitorous Blood should grant trample");
}

// ── Blasphemous Act ────────────────────────────────────────────

/// Blasphemous Act deals 13 damage to each creature.
#[test]
fn blasphemous_act_deals_13_damage_to_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 14);
    let c2 = ready_creature(&mut state, P1, 3, 3);

    // Add tons of mana to afford it even with no cost reduction.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 8);

    let spell = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // c1 has 14 toughness, should have 13 damage.
    assert_eq!(state.get_object(c1).unwrap().damage_marked, 13,
        "Blasphemous Act should deal 13 damage to creature");

    // The 3-toughness creature takes the same 13; nothing here runs SBAs, so it
    // is still on the battlefield holding lethal damage.
    assert_eq!(state.get_object(c2).unwrap().damage_marked, 13,
        "Blasphemous Act should deal 13 damage to opponent's creature too");
}

/// Blasphemous Act cost reduction works.
#[test]
fn blasphemous_act_cost_reduction() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // No creatures: costs {8}{R} = 9 mana.
    let ba = reg.get(reg.get_id_by_name("Blasphemous Act").unwrap()).unwrap();
    assert!(ba.modified_cost(&state, &reg).is_none(),
        "With 0 creatures, no cost modification needed");

    // Add 5 creatures: should cost {3}{R} = 4 mana.
    for _ in 0..5 {
        ready_creature(&mut state, P0, 1, 1);
    }
    let modified = ba.modified_cost(&state, &reg).unwrap();
    assert_eq!(modified.mana_value(), 4, "With 5 creatures, Blasphemous Act should cost {{3}}{{R}}");

    // Add 8+ creatures: should cost {R} = 1 mana.
    for _ in 0..5 {
        ready_creature(&mut state, P1, 1, 1);
    }
    let modified = ba.modified_cost(&state, &reg).unwrap();
    assert_eq!(modified.mana_value(), 1, "With 10 creatures, Blasphemous Act should cost just {{R}}");
}

/// Blasphemous Act can be cast cheaply with many creatures.
#[test]
fn blasphemous_act_castable_with_cost_reduction() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Add 8 creatures so cost is just {R}.
    for _ in 0..4 {
        ready_creature(&mut state, P0, 1, 1);
    }
    for _ in 0..4 {
        ready_creature(&mut state, P1, 1, 1);
    }

    // Give P0 just 1 red mana.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let spell = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);

    // Should be able to cast with just {R}.
    let has_cast = can_cast(&state, &reg, spell);
    assert!(has_cast, "Blasphemous Act should be castable for {{R}} with 8 creatures on the battlefield");
}

// ── Cackling Counterpart ───────────────────────────────────────

/// Cackling Counterpart creates a token copy of target creature you control.
#[test]
fn cackling_counterpart_creates_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let original = named_permanent(&mut state, &reg, "Chapel Geist", P0);

    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(original)]);

    // Should now have 2 Chapel Geists on the battlefield.
    let geists: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Chapel Geist" && o.controller == P0)
        .collect();
    assert_eq!(geists.len(), 2, "Should have original + token copy of Chapel Geist");

    // The token should be a token.
    let token = geists.iter().find(|o| o.is_token).expect("One should be a token");
    assert_eq!(token.power, Some(2));
    assert_eq!(token.toughness, Some(3));
}

// ── Sever the Bloodline ────────────────────────────────────────

/// Sever the Bloodline exiles target creature and all others with the same name.
#[test]
fn sever_the_bloodline_exiles_all_with_same_name() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create 3 creatures with the same name.
    let z1 = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(z1).unwrap().name = "Zombie Token".into();
    let z2 = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(z2).unwrap().name = "Zombie Token".into();
    let z3 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(z3).unwrap().name = "Zombie Token".into();
    // And one with a different name.
    let bear = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(bear).unwrap().name = "Bear".into();

    let spell = castable_spell(&mut state, &reg, "Sever the Bloodline", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(z1)]);

    // All 3 Zombie Tokens should be exiled.
    assert_eq!(state.get_object(z1).unwrap().zone, Zone::Exile, "Target should be exiled");
    assert_eq!(state.get_object(z2).unwrap().zone, Zone::Exile, "Same-name creature should be exiled");
    assert_eq!(state.get_object(z3).unwrap().zone, Zone::Exile, "Own creature with same name should be exiled too");

    // Bear should be unaffected.
    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Battlefield, "Differently-named creature should be unaffected");
}

// ── Angelic Overseer ───────────────────────────────────────────

/// "Flying. As long as you control a Human, Angelic Overseer has hexproof and
/// indestructible." Two of its three keywords come and go with the board; the
/// third must not.
#[test]
fn angelic_overseer_hexproof_indestructible_with_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_permanent(&mut state, &reg, "Angelic Overseer", P0);

    // Without a Human: no hexproof or indestructible.
    assert!(!state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should not have hexproof without a Human");
    assert!(!state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should not be indestructible without a Human");

    // Add a Human.
    let human = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // Now should have hexproof and indestructible.
    assert!(state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should have hexproof when you control a Human");
    assert!(state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should be indestructible when you control a Human");

    // Remove the Human.
    state.move_object(human, Zone::Graveyard, &reg);
    assert!(!state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should lose hexproof when Human leaves");
    assert!(!state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should lose indestructible when Human leaves");

    // Flying is printed, not conditional, so it survives all of that.
    assert!(state.has_keyword(angel, Keyword::Flying, &reg),
        "flying is unconditional — losing the Human must not take it too");
}

/// Angelic Overseer survives destroy effects when indestructible.
#[test]
fn angelic_overseer_survives_destroy_with_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_permanent(&mut state, &reg, "Angelic Overseer", P0);
    let _human = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // Try to destroy the angel.
    let result = mtg_engine::destruction::try_destroy(&mut state, angel, &reg);
    assert_eq!(result, mtg_engine::destruction::DestroyResult::Indestructible,
        "Angelic Overseer should be indestructible when you control a Human");

    // Angel should still be on the battlefield.
    assert_eq!(state.get_object(angel).unwrap().zone, Zone::Battlefield,
        "Angelic Overseer should survive destruction");
}

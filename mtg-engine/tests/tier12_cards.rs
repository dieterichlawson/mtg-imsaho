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

    // Burning Vengeance now presents a target choice. Resolve it by targeting P1.
    assert!(state.awaiting_action.is_some(), "Should be awaiting target choice");
    if let Some(ref choice) = state.awaiting_action {
        // Apply the effect directly for the test.
        let effect = match choice {
            mtg_engine::state::AwaitingAction::ResolutionChoice { choice, .. } => {
                match choice {
                    mtg_engine::state::ResolutionChoiceKind::ChooseTarget { effect, .. } => Some(effect.clone()),
                    _ => None,
                }
            }
            _ => None,
        };
        state.awaiting_action = None;
        if let Some(effect) = effect {
            mtg_engine::engine::apply_pending_effect(&mut state, &mtg_engine::actions::Target::Player(P1), &effect, &reg);
        }
    }

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

/// Traitorous Blood control change reverts at end of turn.
#[test]
fn traitorous_blood_reverts_at_end_of_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enemy = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(enemy).unwrap().name = "Enemy Beast".into();

    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(enemy)]);

    // Verify steal happened.
    assert_eq!(state.get_object(enemy).unwrap().controller, P0);

    // The control change should be tracked for revert.
    assert!(!state.until_end_of_turn_control_changes.is_empty(),
        "Control change should be recorded for end-of-turn revert");
    let (obj_id, original_controller) = state.until_end_of_turn_control_changes[0];
    assert_eq!(obj_id, enemy);
    assert_eq!(original_controller, P1, "Original controller should be recorded as P1");
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

    // c2 had 3 toughness with 13 damage, should be dead after SBAs.
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
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let has_cast = legal.actions.iter().any(|a| matches!(a, mtg_engine::actions::Action::CastSpell { object_id, .. } if *object_id == spell));
    assert!(has_cast, "Blasphemous Act should be castable for {{R}} with 8 creatures on the battlefield");
}

// ── Cackling Counterpart ───────────────────────────────────────

/// Cackling Counterpart creates a token copy of target creature you control.
#[test]
fn cackling_counterpart_creates_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let original = named_creature(&mut state, &reg, "Chapel Geist", P0);

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

/// Cackling Counterpart has flashback.
#[test]
fn cackling_counterpart_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Cackling Counterpart").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some(), "Cackling Counterpart should have flashback");
    assert_eq!(data.flashback_cost.unwrap().mana_value(), 7, "Flashback cost should be {{5}}{{U}}{{U}} = 7");
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

/// Sever the Bloodline has flashback.
#[test]
fn sever_the_bloodline_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Sever the Bloodline").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some(), "Sever the Bloodline should have flashback");
    assert_eq!(data.flashback_cost.unwrap().mana_value(), 7, "Flashback cost should be {{5}}{{B}}{{B}} = 7");
}

// ── Angelic Overseer ───────────────────────────────────────────

/// Angelic Overseer has flying always.
#[test]
fn angelic_overseer_has_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_creature(&mut state, &reg, "Angelic Overseer", P0);
    assert!(state.has_keyword(angel, Keyword::Flying, &reg),
        "Angelic Overseer should always have flying");
}

/// Angelic Overseer gets hexproof and indestructible when you control a Human.
#[test]
fn angelic_overseer_hexproof_indestructible_with_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_creature(&mut state, &reg, "Angelic Overseer", P0);

    // Without a Human: no hexproof or indestructible.
    assert!(!state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should not have hexproof without a Human");
    assert!(!state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should not be indestructible without a Human");

    // Add a Human.
    let human = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    // Now should have hexproof and indestructible.
    assert!(state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should have hexproof when you control a Human");
    assert!(state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should be indestructible when you control a Human");

    // Remove the Human.
    state.move_object(human, Zone::Graveyard);
    assert!(!state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should lose hexproof when Human leaves");
    assert!(!state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should lose indestructible when Human leaves");
}

/// Angelic Overseer survives destroy effects when indestructible.
#[test]
fn angelic_overseer_survives_destroy_with_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_creature(&mut state, &reg, "Angelic Overseer", P0);
    let _human = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    // Try to destroy the angel.
    let result = mtg_engine::destruction::try_destroy(&mut state, angel, &reg);
    assert_eq!(result, mtg_engine::destruction::DestroyResult::Indestructible,
        "Angelic Overseer should be indestructible when you control a Human");

    // Angel should still be on the battlefield.
    assert_eq!(state.get_object(angel).unwrap().zone, Zone::Battlefield,
        "Angelic Overseer should survive destruction");
}

// ═══════════════════════════════════════════════════════════════════
// Ancient Grudge
// ═══════════════════════════════════════════════════════════════════

#[test]
fn ancient_grudge_destroys_artifact() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create an artifact on the battlefield for P1.
    let artifact_card_id = reg.get_id_by_name("Blazing Torch").unwrap();
    let artifact = state.create_object(artifact_card_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(artifact).unwrap().name = "Blazing Torch".into();

    let spell = castable_spell(&mut state, &reg, "Ancient Grudge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(artifact)]);

    assert_eq!(state.get_object(artifact).unwrap().zone, Zone::Graveyard,
        "Ancient Grudge should destroy the artifact");
}

#[test]
fn ancient_grudge_has_flashback() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Ancient Grudge").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert!(data.flashback_cost.is_some(), "Ancient Grudge should have flashback");
    assert_eq!(data.flashback_cost.unwrap().mana_value(), 1, "Flashback should cost G = 1 MV");
}

// ═══════════════════════════════════════════════════════════════════
// Armored Skaab
// ═══════════════════════════════════════════════════════════════════

#[test]
fn armored_skaab_mills_four_on_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 5 cards in P0's library.
    for _ in 0..5 {
        let c = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(c);
    }

    let skaab = named_creature(&mut state, &reg, "Armored Skaab", P0);
    let behavior = reg.get(state.get_object(skaab).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, skaab, &reg);

    // Should have milled 4 cards (1 remaining in library).
    let gy_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0)
        .count();
    assert_eq!(gy_count, 4, "Armored Skaab should mill 4 cards");
    assert_eq!(state.get_player(P0).library_order.len(), 1, "Should have 1 card left in library");
}

#[test]
fn armored_skaab_is_1_4_zombie_warrior() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Armored Skaab").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert_eq!(data.power, Some(1));
    assert_eq!(data.toughness, Some(4));
    assert!(data.subtypes.contains(&"Zombie".into()));
    assert!(data.subtypes.contains(&"Warrior".into()));
}

// ═══════════════════════════════════════════════════════════════════
// Essence of the Wild
// ═══════════════════════════════════════════════════════════════════

#[test]
fn essence_of_the_wild_transforms_entering_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = named_creature(&mut state, &reg, "Essence of the Wild", P0);

    // Create a 1/1 creature entering under P0's control.
    let small = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(small).unwrap().name = "Small Creature".into();

    let behavior = reg.get(state.get_object(essence).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, essence, small, P0, &reg);

    // Small creature should now be a 6/6 copy of Essence.
    let obj = state.get_object(small).unwrap();
    assert_eq!(obj.power, Some(6), "Should be 6 power");
    assert_eq!(obj.toughness, Some(6), "Should be 6 toughness");
    assert_eq!(obj.name, "Essence of the Wild", "Should have Essence's name");
}

#[test]
fn essence_of_the_wild_ignores_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = named_creature(&mut state, &reg, "Essence of the Wild", P0);

    // Create a creature entering under P1's control.
    let opp = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(opp).unwrap().name = "Opponent Creature".into();

    let behavior = reg.get(state.get_object(essence).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, essence, opp, P1, &reg);

    // Opponent's creature should be unchanged.
    let obj = state.get_object(opp).unwrap();
    assert_eq!(obj.power, Some(2), "Opponent's creature should be unchanged");
    assert_eq!(obj.toughness, Some(2), "Opponent's creature should be unchanged");
}

// ═══════════════════════════════════════════════════════════════════
// Selhoff Occultist
// ═══════════════════════════════════════════════════════════════════

#[test]
fn selhoff_occultist_mills_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let occultist = named_creature(&mut state, &reg, "Selhoff Occultist", P0);

    // Put a card in P1's library to mill.
    let lib_card = state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Library, None, None);
    state.get_player_mut(P1).library_order.push(lib_card);

    // Trigger on another creature dying.
    let dead = ready_creature(&mut state, P1, 1, 1);
    let behavior = reg.get(state.get_object(occultist).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, occultist, dead, P1, &[], 1, &reg);

    // Should have a choice to target a player for milling.
    assert!(state.awaiting_action.is_some(), "Should have a pending target choice");

    // Choose P1 as the target.
    state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(
                mtg_engine::actions::Target::Player(P1),
            )),
        },
        &reg,
    );

    // P1 should have milled 1 card.
    let gy_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert!(gy_count >= 1, "P1 should have milled at least 1 card");
}

#[test]
fn selhoff_occultist_is_2_3_human_rogue() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Selhoff Occultist").unwrap();
    let data = reg.card_data(card_id).unwrap();
    assert_eq!(data.power, Some(2));
    assert_eq!(data.toughness, Some(3));
    assert!(data.subtypes.contains(&"Human".into()));
    assert!(data.subtypes.contains(&"Rogue".into()));
}

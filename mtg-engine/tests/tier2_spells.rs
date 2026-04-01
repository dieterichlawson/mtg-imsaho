//! Tests for Innistrad Tier 2 cards: targeted removal, bounce, fight,
//! permanent destruction, and counter variants.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Simple damage spells ────────────────────────────────────────────

#[test]
fn bump_in_the_night_drains_3() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = castable_spell(&mut state, &reg, "Bump in the Night", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, 17);
}

#[test]
fn geistflame_deals_1_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    let card = castable_spell(&mut state, &reg, "Geistflame", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().damage_marked, 1);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "2/2 with 1 damage should survive");
}

#[test]
fn brimstone_volley_deals_3_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = castable_spell(&mut state, &reg, "Brimstone Volley", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, 17);
}

// ── Counter variants ────────────────────────────────────────────────

/// Dissipate counters and exiles the spell (not graveyard).
#[test]
fn dissipate_counters_and_exiles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts a creature spell.
    let tusker = castable_spell(&mut state, &reg, "Kalonian Tusker", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: tusker, targets: vec![], sacrifice: None },
        &reg,
    );

    // P1 casts Dissipate targeting the Tusker on the stack.
    let diss = castable_spell(&mut state, &reg, "Dissipate", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, diss, vec![Target::Object(tusker)]);

    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Exile,
        "Dissipate should exile the countered spell, not put it in graveyard");
    assert_eq!(state.get_object(diss).unwrap().zone, Zone::Graveyard);
}

/// Frightful Delusion counters and forces a discard.
#[test]
fn frightful_delusion_counters_and_discards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a card in hand (to be discarded).
    let hand_card = spell_in_hand(&mut state, &reg, "Mountain", P0);

    // P0 casts a creature.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![], sacrifice: None },
        &reg,
    );

    // P1 casts Frightful Delusion.
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    // P0's hand card should have been discarded.
    assert_eq!(state.get_object(hand_card).unwrap().zone, Zone::Graveyard,
        "Controller of countered spell should discard a card");
}

// ── Creature-type filtered removal ──────────────────────────────────

/// Victim of Night destroys a non-Vampire/Werewolf/Zombie creature.
#[test]
fn victim_of_night_kills_normal_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = ready_creature(&mut state, P1, 2, 2);
    let card = castable_spell(&mut state, &reg, "Victim of Night", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(bears)]);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard);
}

/// Victim of Night can't target a Vampire (Markov Patrician).
#[test]
fn victim_of_night_cant_target_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let vamp = named_creature(&mut state, &reg, "Markov Patrician", P1);

    let _card = castable_spell(&mut state, &reg, "Victim of Night", P0);

    let legal = engine::legal_actions(&state, &reg);
    let targets_vamp = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == vamp)))
    });
    assert!(!targets_vamp,
        "Victim of Night should not be able to target a Vampire");
}

/// Smite the Monstrous destroys creature with power 4+.
#[test]
fn smite_the_monstrous_kills_big_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let big = ready_creature(&mut state, P1, 5, 5);
    let small = ready_creature(&mut state, P1, 2, 2);

    let card = castable_spell(&mut state, &reg, "Smite the Monstrous", P0);

    // Should be able to target the 5/5 but not the 2/2.
    let legal = engine::legal_actions(&state, &reg);
    let targets_big = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == big)))
    });
    let targets_small = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == small)))
    });
    assert!(targets_big, "Should be able to target 5/5");
    assert!(!targets_small, "Should not be able to target 2/2");

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(big)]);

    assert_eq!(state.get_object(big).unwrap().zone, Zone::Graveyard);
}

/// Rebuke destroys an attacking creature.
#[test]
fn rebuke_destroys_attacking_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let non_attacker = ready_creature(&mut state, P0, 2, 2);

    // Declare attacker.
    combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.priority_player = Some(P1);

    // P1 casts Rebuke.
    let card = castable_spell(&mut state, &reg, "Rebuke", P1);

    // Rebuke should only target the attacking creature.
    let legal = engine::legal_actions(&state, &reg);
    let targets_attacker = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == attacker)))
    });
    let targets_non_attacker = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == non_attacker)))
    });
    assert!(targets_attacker, "Should target the attacking creature");
    assert!(!targets_non_attacker, "Should not target non-attacking creature");

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(attacker)]);

    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Graveyard);
}

// ── Bounce ──────────────────────────────────────────────────────────

/// Silent Departure returns a creature to its owner's hand.
#[test]
fn silent_departure_bounces_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let card = castable_spell(&mut state, &reg, "Silent Departure", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Creature should be returned to hand");
}

// ── Permanent removal ───────────────────────────────────────────────

/// Naturalize destroys an enchantment.
#[test]
fn naturalize_destroys_enchantment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put an enchantment on the battlefield.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let pac = castable_spell(&mut state, &reg, "Pacifism", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, pac, vec![Target::Object(creature)]);
    assert_eq!(state.get_object(pac).unwrap().zone, Zone::Battlefield);

    // P0 casts Naturalize on the Pacifism.
    state.priority_player = Some(P0);
    let nat = castable_spell(&mut state, &reg, "Naturalize", P0);

    state = cast_and_resolve(&state, &reg, nat, vec![Target::Object(pac)]);

    assert_eq!(state.get_object(pac).unwrap().zone, Zone::Graveyard,
        "Naturalize should destroy the enchantment");
}

/// Naturalize can't target a creature (only artifacts/enchantments).
#[test]
fn naturalize_cant_target_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let _nat = castable_spell(&mut state, &reg, "Naturalize", P0);

    let legal = engine::legal_actions(&state, &reg);
    let targets_creature = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == creature)))
    });
    assert!(!targets_creature, "Naturalize should not target a creature");
}

/// Bramblecrush destroys a noncreature permanent (e.g., a land).
#[test]
fn bramblecrush_destroys_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let land = state.create_object(forest_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(land).unwrap().name = "Forest".into();
    state.get_object_mut(land).unwrap().summoning_sick = false;

    let bc = castable_spell(&mut state, &reg, "Bramblecrush", P0);

    state = cast_and_resolve(&state, &reg, bc, vec![Target::Object(land)]);

    assert_eq!(state.get_object(land).unwrap().zone, Zone::Graveyard,
        "Bramblecrush should destroy the land");
}

/// Bramblecrush can't target a creature.
#[test]
fn bramblecrush_cant_target_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let _bc = castable_spell(&mut state, &reg, "Bramblecrush", P0);

    let legal = engine::legal_actions(&state, &reg);
    let targets_creature = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == creature)))
    });
    assert!(!targets_creature, "Bramblecrush should not target a creature");
}

/// Urgent Exorcism destroys a Spirit creature.
#[test]
fn urgent_exorcism_destroys_spirit() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Chapel Geist is a Spirit.
    let geist = named_creature(&mut state, &reg, "Chapel Geist", P1);

    let ue = castable_spell(&mut state, &reg, "Urgent Exorcism", P0);

    state = cast_and_resolve(&state, &reg, ue, vec![Target::Object(geist)]);

    assert_eq!(state.get_object(geist).unwrap().zone, Zone::Graveyard,
        "Urgent Exorcism should destroy a Spirit");
}

// ── Fight ───────────────────────────────────────────────────────────

/// Prey Upon: your creature fights their creature. Both deal damage.
#[test]
fn prey_upon_fight() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(mine).unwrap().controller = P0;
    let theirs = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(theirs).unwrap().controller = P1;

    let pu = castable_spell(&mut state, &reg, "Prey Upon", P0);

    state = cast_and_resolve(&state, &reg, pu, vec![Target::Object(mine), Target::Object(theirs)]);

    // 3/3 deals 3 to 2/2, 2/2 deals 2 to 3/3.
    assert_eq!(state.get_object(mine).unwrap().damage_marked, 2);
    assert_eq!(state.get_object(theirs).unwrap().damage_marked, 3);

    // SBA kills the 2/2.
    check_state_based_actions_with_registry(&mut state, Some(&reg));
    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Battlefield);
}

// ── Two-target spells ───────────────────────────────────────────────

/// Lost in the Mist counters a spell and bounces a permanent.
#[test]
fn lost_in_the_mist_counters_and_bounces() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature on the battlefield.
    let creature = ready_creature(&mut state, P0, 3, 3);

    // P0 casts a spell.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![], sacrifice: None },
        &reg,
    );

    // P1 casts Lost in the Mist targeting the spell + the creature.
    let litm = castable_spell(&mut state, &reg, "Lost in the Mist", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, litm, vec![Target::Object(bears), Target::Object(creature)]);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Permanent should be bounced to hand");
}

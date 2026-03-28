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

    let card_id = reg.get_id_by_name("Bump in the Night").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Bump in the Night".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Player(P1)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 17);
}

#[test]
fn geistflame_deals_1_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    let card_id = reg.get_id_by_name("Geistflame").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Geistflame".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().damage_marked, 1);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "2/2 with 1 damage should survive");
}

#[test]
fn brimstone_volley_deals_3_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Brimstone Volley").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Brimstone Volley".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 3);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Player(P1)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 17);
}

// ── Counter variants ────────────────────────────────────────────────

/// Dissipate counters and exiles the spell (not graveyard).
#[test]
fn dissipate_counters_and_exiles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts a creature spell.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let tusker = state.create_object(tusker_id, P0, Zone::Hand, None, None);
    state.get_object_mut(tusker).unwrap().name = "Kalonian Tusker".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: tusker, targets: vec![] },
        &reg,
    );

    // P1 casts Dissipate targeting the Tusker on the stack.
    let diss_id = reg.get_id_by_name("Dissipate").unwrap();
    let diss = state.create_object(diss_id, P1, Zone::Hand, None, None);
    state.get_object_mut(diss).unwrap().name = "Dissipate".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::Blue, 3);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: diss, targets: vec![Target::Object(tusker)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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
    let mountain_id = reg.get_id_by_name("Mountain").unwrap();
    let hand_card = state.create_object(mountain_id, P0, Zone::Hand, None, None);
    state.get_object_mut(hand_card).unwrap().name = "Mountain".into();

    // P0 casts a creature.
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![] },
        &reg,
    );

    // P1 casts Frightful Delusion.
    let fd_id = reg.get_id_by_name("Frightful Delusion").unwrap();
    let fd = state.create_object(fd_id, P1, Zone::Hand, None, None);
    state.get_object_mut(fd).unwrap().name = "Frightful Delusion".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::Blue, 3);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: fd, targets: vec![Target::Object(bears)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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
    let card_id = reg.get_id_by_name("Victim of Night").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Victim of Night".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Object(bears)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard);
}

/// Victim of Night can't target a Vampire (Markov Patrician).
#[test]
fn victim_of_night_cant_target_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let vamp_id = reg.get_id_by_name("Markov Patrician").unwrap();
    let vamp = state.create_object(vamp_id, P1, Zone::Battlefield, Some(3), Some(1));
    state.get_object_mut(vamp).unwrap().name = "Markov Patrician".into();
    state.get_object_mut(vamp).unwrap().summoning_sick = false;

    let card_id = reg.get_id_by_name("Victim of Night").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Victim of Night".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

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

    let card_id = reg.get_id_by_name("Smite the Monstrous").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Smite the Monstrous".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 4);

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

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Object(big)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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
    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    state.priority_player = Some(P1);

    // P1 casts Rebuke.
    let card_id = reg.get_id_by_name("Rebuke").unwrap();
    let card = state.create_object(card_id, P1, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Rebuke".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::White, 3);

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

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Object(attacker)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Graveyard);
}

// ── Bounce ──────────────────────────────────────────────────────────

/// Silent Departure returns a creature to its owner's hand.
#[test]
fn silent_departure_bounces_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let card_id = reg.get_id_by_name("Silent Departure").unwrap();
    let card = state.create_object(card_id, P0, Zone::Hand, None, None);
    state.get_object_mut(card).unwrap().name = "Silent Departure".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: card, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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
    let pac_id = reg.get_id_by_name("Pacifism").unwrap();
    let pac = state.create_object(pac_id, P1, Zone::Hand, None, None);
    state.get_player_mut(P1).mana_pool.add(ManaType::White, 2);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: pac, targets: vec![Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_object(pac).unwrap().zone, Zone::Battlefield);

    // P0 casts Naturalize on the Pacifism.
    state.priority_player = Some(P0);
    let nat_id = reg.get_id_by_name("Naturalize").unwrap();
    let nat = state.create_object(nat_id, P0, Zone::Hand, None, None);
    state.get_object_mut(nat).unwrap().name = "Naturalize".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: nat, targets: vec![Target::Object(pac)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(pac).unwrap().zone, Zone::Graveyard,
        "Naturalize should destroy the enchantment");
}

/// Naturalize can't target a creature (only artifacts/enchantments).
#[test]
fn naturalize_cant_target_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let nat_id = reg.get_id_by_name("Naturalize").unwrap();
    let nat = state.create_object(nat_id, P0, Zone::Hand, None, None);
    state.get_object_mut(nat).unwrap().name = "Naturalize".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

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

    let bc_id = reg.get_id_by_name("Bramblecrush").unwrap();
    let bc = state.create_object(bc_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bc).unwrap().name = "Bramblecrush".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 4);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bc, targets: vec![Target::Object(land)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(land).unwrap().zone, Zone::Graveyard,
        "Bramblecrush should destroy the land");
}

/// Bramblecrush can't target a creature.
#[test]
fn bramblecrush_cant_target_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let bc_id = reg.get_id_by_name("Bramblecrush").unwrap();
    let bc = state.create_object(bc_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bc).unwrap().name = "Bramblecrush".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 4);

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
    let geist_id = reg.get_id_by_name("Chapel Geist").unwrap();
    let geist = state.create_object(geist_id, P1, Zone::Battlefield, Some(2), Some(3));
    state.get_object_mut(geist).unwrap().name = "Chapel Geist".into();
    state.get_object_mut(geist).unwrap().summoning_sick = false;

    let ue_id = reg.get_id_by_name("Urgent Exorcism").unwrap();
    let ue = state.create_object(ue_id, P0, Zone::Hand, None, None);
    state.get_object_mut(ue).unwrap().name = "Urgent Exorcism".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: ue, targets: vec![Target::Object(geist)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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

    let pu_id = reg.get_id_by_name("Prey Upon").unwrap();
    let pu = state.create_object(pu_id, P0, Zone::Hand, None, None);
    state.get_object_mut(pu).unwrap().name = "Prey Upon".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: pu, targets: vec![Target::Object(mine), Target::Object(theirs)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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
    let bears_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let bears = state.create_object(bears_id, P0, Zone::Hand, None, None);
    state.get_object_mut(bears).unwrap().name = "Grizzly Bears".into();
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 2);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![] },
        &reg,
    );

    // P1 casts Lost in the Mist targeting the spell + the creature.
    let litm_id = reg.get_id_by_name("Lost in the Mist").unwrap();
    let litm = state.create_object(litm_id, P1, Zone::Hand, None, None);
    state.get_object_mut(litm).unwrap().name = "Lost in the Mist".into();
    state.get_player_mut(P1).mana_pool.add(ManaType::Blue, 5);
    state.priority_player = Some(P1);

    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: litm, targets: vec![Target::Object(bears), Target::Object(creature)] },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Permanent should be bounced to hand");
}

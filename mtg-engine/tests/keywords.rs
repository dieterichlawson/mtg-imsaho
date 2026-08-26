//! Tests for keyword abilities: flying, first strike, trample, deathtouch,
//! lifelink, vigilance, flash, reach, haste, defender, hexproof, intimidate.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
// ── Flying ──────────────────────────────────────────────────────────

/// A creature with flying can only be blocked by creatures with flying or reach.
/// CR 509.1b: a creature with flying can't be blocked except by creatures with
/// flying or reach. Four tests used to take one cell of this each.
#[test]
fn flying_restricts_who_can_block() {
    // (attacker, blocker, blocker may block the attacker?)
    const CASES: &[(&str, &str, bool)] = &[
        ("Abbey Griffin", "Grizzly Bears",     false), // flyer vs ground: no
        ("Abbey Griffin", "Moon Heron",        true),  // flyer vs flyer: yes
        ("Moon Heron",    "Somberwald Spider", true),  // flyer vs reach: yes
        ("Grizzly Bears", "Grizzly Bears",     true),  // ground attacker: unrestricted
    ];
    let reg = registry();
    for &(attacker_name, blocker_name, can_block) in CASES {
        let mut state = game_at_step(Step::DeclareBlockers, P0);
        let attacker = named_permanent(&mut state, &reg, attacker_name, P0);
        let blocker = named_permanent(&mut state, &reg, blocker_name, P1);

        assert_eq!(combat::can_block_attacker(&state, blocker, attacker, &reg), can_block,
            "{blocker_name} should {}be able to block {attacker_name}",
            if can_block { "" } else { "not " });
    }
}

/// A creature with flying CAN be blocked by another creature with flying.
/// A creature with reach can block a flyer.
/// A non-flying attacker CAN be blocked by any creature (flying doesn't restrict ground blocks).
// ── Vigilance ───────────────────────────────────────────────────────

/// Creatures with vigilance don't tap when attacking.
#[test]
fn vigilance_does_not_tap_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = named_permanent(&mut state, &reg, "Abbey Griffin", P0);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    assert!(!state.get_object(attacker).unwrap().tapped,
        "Creature with vigilance should not be tapped after attacking");
}

/// A creature without vigilance still taps when attacking.
#[test]
fn non_vigilance_taps_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    assert!(state.get_object(attacker).unwrap().tapped,
        "Creature without vigilance should tap when attacking");
}

// ── Defender ────────────────────────────────────────────────────────

/// Creatures with defender cannot attack.
#[test]
fn defender_cannot_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let defender = named_permanent(&mut state, &reg, "Grave Bramble", P0);

    let eligible = combat::eligible_attackers(&state, P0, &reg);
    assert!(!eligible.contains(&defender),
        "Creature with defender should not be eligible to attack");
}

/// Creatures with defender CAN still block.
#[test]
fn defender_can_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let defender = named_permanent(&mut state, &reg, "Grave Bramble", P1);

    let eligible = combat::eligible_blockers(&state, P1, &reg);
    assert!(eligible.contains(&defender),
        "Creature with defender should still be eligible to block");
}

// ── Haste ───────────────────────────────────────────────────────────

/// Creatures with haste can attack the turn they enter (no summoning sickness).
#[test]
fn haste_overrides_summoning_sickness() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    // Simulate a creature with haste that just entered (summoning_sick = true).
    // Use a creature object with haste keyword.
    // We don't have a haste creature card yet, so manually set the keyword.
    let creature = sick_creature(&mut state, P0, 3, 1);
    assert!(state.get_object(creature).unwrap().summoning_sick);

    // Without haste, should not be eligible.
    let eligible = combat::eligible_attackers(&state, P0, &reg);
    assert!(!eligible.contains(&creature));

    // Mark it as having haste via until-end-of-turn keyword.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword {
            target: creature,
            keyword: Keyword::Haste,
        }
    );

    let eligible = combat::eligible_attackers(&state, P0, &reg);
    assert!(eligible.contains(&creature),
        "Creature with haste should be able to attack despite summoning sickness");
}

// ── Hexproof ────────────────────────────────────────────────────────

/// Creatures with hexproof can't be targeted by opponents' spells.
#[test]
fn hexproof_prevents_opponent_targeting() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has Invisible Stalker on battlefield.
    let stalker = named_permanent(&mut state, &reg, "Invisible Stalker", P1);

    // P0 has Lightning Bolt in hand with mana to cast it.
    let _bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    let legal = engine::legal_actions(&state, &reg);

    // There should NOT be a CastSpell action targeting the Stalker.
    let targets_stalker = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == stalker)))
    });
    assert!(!targets_stalker,
        "Lightning Bolt should not be able to target a hexproof creature controlled by opponent");

    // But P0 CAN target their own hexproof creature (hexproof only stops opponents).
    let stalker_p0 = named_permanent(&mut state, &reg, "Invisible Stalker", P0);

    let legal2 = engine::legal_actions(&state, &reg);
    let targets_own = legal2.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { targets, .. }
            if targets.iter().any(|t| matches!(t, Target::Object(id) if *id == stalker_p0)))
    });
    assert!(targets_own,
        "Should be able to target own hexproof creature");
}

// ── Intimidate ──────────────────────────────────────────────────────

/// Creatures with intimidate can only be blocked by artifact creatures or
/// creatures that share a color.
#[test]
fn intimidate_blocks_different_color() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // Spectral Rider is white with intimidate.
    let attacker = named_permanent(&mut state, &reg, "Spectral Rider", P0);
    state.get_object_mut(attacker).unwrap().colors = vec![Color::White];

    // Green creature can't block it.
    let green_blocker = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(green_blocker).unwrap().colors = vec![Color::Green];

    assert!(!combat::can_block_attacker(&state, green_blocker, attacker, &reg),
        "Green creature should not block a white intimidate creature");

    // White creature CAN block it (shares color).
    let white_blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(white_blocker).unwrap().colors = vec![Color::White];

    assert!(combat::can_block_attacker(&state, white_blocker, attacker, &reg),
        "White creature should be able to block a white intimidate creature");
}

/// Artifact creatures can block creatures with intimidate regardless of color.
#[test]
fn artifact_creature_blocks_intimidate() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = named_permanent(&mut state, &reg, "Spectral Rider", P0);
    state.get_object_mut(attacker).unwrap().colors = vec![Color::White];

    // One-Eyed Scarecrow is an artifact creature.
    let blocker = named_permanent(&mut state, &reg, "One-Eyed Scarecrow", P1);

    assert!(combat::can_block_attacker(&state, blocker, attacker, &reg),
        "Artifact creature should be able to block an intimidate creature");
}

// ── Deathtouch ──────────────────────────────────────────────────────

/// Any damage from a deathtouch creature is lethal (SBA destroys the target).
#[test]
fn deathtouch_kills_with_one_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Typhoid Rats (1/1 deathtouch) attacks, blocked by a 5/5.
    let attacker = named_permanent(&mut state, &reg, "Typhoid Rats", P0);

    let blocker = ready_creature(&mut state, P1, 5, 5);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    // Blocker took 1 damage from deathtouch source.
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 1);
    assert!(state.get_object(blocker).unwrap().dealt_deathtouch_damage);

    // SBA should kill it despite only 1 damage on a 5-toughness creature.
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard,
        "Creature dealt deathtouch damage should die regardless of toughness");
}

/// Deathtouch with trample: only 1 damage needed per blocker, rest tramples through.
#[test]
fn deathtouch_trample_assigns_minimum() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // 5/5 with deathtouch + trample (simulated via EOT keywords).
    let attacker = ready_creature(&mut state, P0, 5, 5);
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword { target: attacker, keyword: Keyword::Deathtouch }
    );
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword { target: attacker, keyword: Keyword::Trample }
    );

    let blocker = ready_creature(&mut state, P1, 2, 4);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    // With deathtouch + trample: 1 damage assigned to blocker (lethal), 4 tramples through.
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 1);
    assert_eq!(state.get_player(P1).life, 16,
        "4 damage should trample to defending player (5 power - 1 deathtouch lethal)");
}

// ── Lifelink ────────────────────────────────────────────────────────

/// Lifelink: controller gains life equal to combat damage dealt.
#[test]
fn lifelink_gains_life_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = named_permanent(&mut state, &reg, "Markov Patrician", P0);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 17,
        "Defending player should take 3 damage");
    assert_eq!(state.get_player(P0).life, 23,
        "Attacking player should gain 3 life from lifelink");
}

/// Lifelink also works when dealing damage to creatures in combat.
#[test]
fn lifelink_gains_life_from_creature_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = named_permanent(&mut state, &reg, "Markov Patrician", P0);

    let blocker = ready_creature(&mut state, P1, 1, 4);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 23,
        "Should gain 3 life from lifelink even when blocked");
}

// ── Trample ─────────────────────────────────────────────────────────

/// Trample: excess damage carries over to the defending player.
#[test]
fn trample_excess_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // 5/5 with trample.
    let attacker = ready_creature(&mut state, P0, 5, 5);
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword { target: attacker, keyword: Keyword::Trample }
    );

    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    // Blocker takes 2 (lethal), remaining 3 tramples to player.
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2);
    assert_eq!(state.get_player(P1).life, 17,
        "3 excess damage should trample through to the player");
}

/// Without trample, all damage goes to blocker even if overkill.
#[test]
fn without_trample_no_excess_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = ready_creature(&mut state, P0, 5, 5);
    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    // All damage goes to blocker.
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 5);
    assert_eq!(state.get_player(P1).life, 20,
        "No damage should carry to player without trample");
}

// ── First strike ────────────────────────────────────────────────────

/// First strike creature deals damage first, killing a blocker before it can hit back.
#[test]
fn first_strike_kills_before_normal_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Voiceless Spirit: 2/1 flying, first strike
    let attacker = named_permanent(&mut state, &reg, "Voiceless Spirit", P0);

    // Blocker: Moon Heron 3/2 (would kill the 2/1 in simultaneous damage, but first strike prevents it)
    let blocker = named_permanent(&mut state, &reg, "Moon Heron", P1);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    // First strike deals 2 to Moon Heron (toughness 2), kills it in first strike step.
    // Then in normal damage step, Moon Heron is dead and doesn't deal damage back.
    // So Voiceless Spirit should survive.
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard,
        "Blocker should die from first strike damage");
    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Battlefield,
        "First striker should survive because blocker dies before dealing normal damage");
}

// ── Flash ───────────────────────────────────────────────────────────

/// Creatures with flash can be cast during the opponent's turn.
#[test]
fn flash_creature_castable_at_instant_speed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P1 priority during P0's turn and an Ambush Viper in hand.
    state.priority_player = Some(P1);
    let viper = castable_spell(&mut state, &reg, "Ambush Viper", P1);

    let legal = engine::legal_actions(&state, &reg);

    let can_cast = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == viper)
    });
    assert!(can_cast,
        "Ambush Viper with flash should be castable during opponent's main phase");
}

/// A normal creature CANNOT be cast during the opponent's turn.
#[test]
fn normal_creature_not_castable_on_opponent_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.priority_player = Some(P1);
    let rats = castable_spell(&mut state, &reg, "Typhoid Rats", P1);

    let legal = engine::legal_actions(&state, &reg);

    let can_cast = legal.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == rats)
    });
    assert!(!can_cast,
        "Normal creature should not be castable during opponent's turn");
}

// ── Blocker validation with flying ──────────────────────────────────

/// Declaring a ground creature as blocker of a flyer is filtered out.
#[test]
fn blocker_validation_rejects_ground_blocking_flyer() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = named_permanent(&mut state, &reg, "Moon Heron", P0);

    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    // Try to illegally block — should be filtered out.
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    // The illegal block was rejected, so the flyer is unblocked → 3 damage to player.
    assert_eq!(state.get_player(P1).life, 17,
        "Illegal block should be filtered out; flyer deals damage unblocked");
}

// ── has_keyword with aura-granted keywords ──────────────────────────

/// Spectral Flight grants flying to the enchanted creature.
#[test]
fn aura_grants_keyword() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    let sf = castable_spell(&mut state, &reg, "Spectral Flight", P0);

    state = cast_and_resolve(&state, &reg, sf, vec![Target::Object(creature)]);

    assert!(state.has_keyword(creature, Keyword::Flying, &reg),
        "Creature with Spectral Flight should have flying");

    // Also check P/T bonus.
    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));
}

// ── has_keyword with until-EOT grants ───────────────────────────────

/// Moment of Heroism grants lifelink until end of turn.
#[test]
fn spell_grants_keyword_until_eot() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let moh = castable_spell(&mut state, &reg, "Moment of Heroism", P0);

    state = cast_and_resolve(&state, &reg, moh, vec![Target::Object(creature)]);

    assert!(state.has_keyword(creature, Keyword::Lifelink, &reg),
        "Creature should have lifelink after Moment of Heroism");
    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));

    advance_to_cleanup(&mut state, &reg);

    assert!(!state.has_keyword(creature, Keyword::Lifelink, &reg),
        "Lifelink should expire at end of turn");
    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "+2/+2 should expire at end of turn");
}

//! Tests for Innistrad Tier 0-1 cards: vanilla creatures, keyword creatures,
//! combat instants, and aura enchantments.
//!
//! Cards covered (16), so this is greppable by name as well as by rule:
//!
//! - Bonds of Faith
//! - Claustrophobia
//! - Dead Weight
//! - Diregraf Ghoul
//! - Furor of the Bitten
//! - Ghostly Possession
//! - Gruesome Deformity
//! - Hysterical Blindness
//! - Markov Patrician
//! - Rally the Peasants
//! - Ranger's Guile
//! - Sensory Deprivation
//! - Skeletal Grimace
//! - Spectral Flight
//! - Spidery Grasp
//! - Vampiric Fury

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
use mtg_engine::triggers;
// ── Diregraf Ghoul enters tapped ────────────────────────────────────

#[test]
fn diregraf_ghoul_enters_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = castable_spell(&mut state, &reg, "Diregraf Ghoul", P0);

    state = cast_and_resolve(&state, &reg, ghoul, vec![]);

    assert_eq!(state.get_object(ghoul).unwrap().zone, Zone::Battlefield);
    assert!(state.get_object(ghoul).unwrap().tapped,
        "Diregraf Ghoul should enter the battlefield tapped");
}

// ── Combat instant spells ───────────────────────────────────────────

/// Rally the Peasants gives all your creatures +2/+0 until end of turn.
#[test]
fn rally_the_peasants_buffs_all_your_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 2);
    let c2 = ready_creature(&mut state, P0, 1, 1);
    let opp = ready_creature(&mut state, P1, 3, 3);

    let rally = castable_spell(&mut state, &reg, "Rally the Peasants", P0);

    state = cast_and_resolve(&state, &reg, rally, vec![]);

    assert_eq!(state.effective_power(c1, &reg), Some(4));
    assert_eq!(state.effective_power(c2, &reg), Some(3));
    // Toughness should be unchanged.
    assert_eq!(state.effective_toughness(c1, &reg), Some(2));
    // Opponent's creature should NOT be affected.
    assert_eq!(state.effective_power(opp, &reg), Some(3));
}

/// Hysterical Blindness gives opponents' creatures -4/-0 until end of turn.
#[test]
fn hysterical_blindness_debuffs_opponents() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 2, 2);
    let opp = ready_creature(&mut state, P1, 5, 5);

    let hb = castable_spell(&mut state, &reg, "Hysterical Blindness", P0);

    state = cast_and_resolve(&state, &reg, hb, vec![]);

    assert_eq!(state.effective_power(opp, &reg), Some(1),
        "Opponent's 5/5 should become 1/5 from -4/-0");
    assert_eq!(state.effective_toughness(opp, &reg), Some(5),
        "Toughness should be unaffected");
    assert_eq!(state.effective_power(mine, &reg), Some(2),
        "Your own creatures should be unaffected");
}

/// Ranger's Guile gives +1/+1 and hexproof until end of turn.
#[test]
fn rangers_guile_gives_hexproof_and_pump() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let rg = castable_spell(&mut state, &reg, "Ranger's Guile", P0);

    state = cast_and_resolve(&state, &reg, rg, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));
    assert!(state.has_keyword(creature, Keyword::Hexproof, &reg));
}

/// Spidery Grasp untaps, gives +2/+4 and reach.
#[test]
fn spidery_grasp_untaps_and_buffs() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().tapped = true;

    let sg = castable_spell(&mut state, &reg, "Spidery Grasp", P0);

    state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);

    assert!(!state.get_object(creature).unwrap().tapped, "Should be untapped");
    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(6));
    assert!(state.has_keyword(creature, Keyword::Reach, &reg));
}

// ── Aura enchantments ───────────────────────────────────────────────

/// Dead Weight gives -2/-2, can kill a creature.
#[test]
fn dead_weight_kills_small_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // 2/2 creature gets Dead Weight (-2/-2) → effective 0/0 → dies to SBA.
    let creature = ready_creature(&mut state, P1, 2, 2);
    let dw = castable_spell(&mut state, &reg, "Dead Weight", P0);

    state = cast_and_resolve(&state, &reg, dw, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(0));
    assert_eq!(state.effective_toughness(creature, &reg), Some(0));

    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature with 0 toughness from Dead Weight should die");
}

/// Sensory Deprivation gives -3/-0.
#[test]
fn sensory_deprivation_reduces_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    let sd = castable_spell(&mut state, &reg, "Sensory Deprivation", P0);

    state = cast_and_resolve(&state, &reg, sd, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(0));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Toughness should be unchanged");
}

/// Spectral Flight grants +2/+2 and flying.
#[test]
fn spectral_flight_buffs_and_grants_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sf = castable_spell(&mut state, &reg, "Spectral Flight", P0);

    state = cast_and_resolve(&state, &reg, sf, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(4));
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));
    assert!(state.has_keyword(creature, Keyword::Flying, &reg),
        "Spectral Flight should grant flying");
}

/// Gruesome Deformity grants intimidate.
#[test]
fn gruesome_deformity_grants_intimidate() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let gd = castable_spell(&mut state, &reg, "Gruesome Deformity", P0);

    state = cast_and_resolve(&state, &reg, gd, vec![Target::Object(creature)]);

    assert!(state.has_keyword(creature, Keyword::Intimidate, &reg),
        "Gruesome Deformity should grant intimidate");
}

/// Bonds of Faith prevents attacking and blocking (simplified non-Human mode).
#[test]
fn bonds_of_faith_prevents_attack_and_block() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    let bof = castable_spell(&mut state, &reg, "Bonds of Faith", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, bof, vec![Target::Object(creature)]);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert!(!state.can_attack(creature, &reg), "Should not be able to attack");
    assert!(!state.can_block(creature, &reg), "Should not be able to block");
}

/// Claustrophobia taps the creature and keeps it tapped.
#[test]
fn claustrophobia_taps_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    assert!(!state.get_object(creature).unwrap().tapped);

    let cl = castable_spell(&mut state, &reg, "Claustrophobia", P0);

    state = cast_and_resolve(&state, &reg, cl, vec![Target::Object(creature)]);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert!(state.get_object(creature).unwrap().tapped,
        "Claustrophobia should tap the enchanted creature on entry");
    assert_eq!(state.get_object(cl).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(cl).unwrap().attached_to, Some(creature));
}

/// Skeletal Grimace gives +1/+1.
#[test]
fn skeletal_grimace_gives_plus_one_plus_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sg = castable_spell(&mut state, &reg, "Skeletal Grimace", P0);

    state = cast_and_resolve(&state, &reg, sg, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));
}

/// Furor of the Bitten gives +2/+2.
#[test]
fn furor_of_the_bitten_gives_plus_two() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 1, 1);
    let fb = castable_spell(&mut state, &reg, "Furor of the Bitten", P0);

    state = cast_and_resolve(&state, &reg, fb, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(3));
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));
}

/// Ghostly Possession grants flying.
#[test]
fn ghostly_possession_grants_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let gp = castable_spell(&mut state, &reg, "Ghostly Possession", P0);

    state = cast_and_resolve(&state, &reg, gp, vec![Target::Object(creature)]);

    assert!(state.has_keyword(creature, Keyword::Flying, &reg),
        "Ghostly Possession should grant flying");
}

// ── Vampiric Fury (tribal instant) ──────────────────────────────────

/// Vampiric Fury gives all your vampires +2/+0 and first strike until EOT.
#[test]
fn vampiric_fury_buffs_vampires() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Markov Patrician is a vampire.
    let vamp = named_permanent(&mut state, &reg, "Markov Patrician", P0);

    // Non-vampire creature.
    let nonvamp = ready_creature(&mut state, P0, 2, 2);

    let vf = castable_spell(&mut state, &reg, "Vampiric Fury", P0);

    state = cast_and_resolve(&state, &reg, vf, vec![]);

    // Vampire should get +2/+0 and first strike.
    assert_eq!(state.effective_power(vamp, &reg), Some(5));
    assert!(state.has_keyword(vamp, Keyword::FirstStrike, &reg));

    // Non-vampire should NOT be affected.
    assert_eq!(state.effective_power(nonvamp, &reg), Some(2));
    assert!(!state.has_keyword(nonvamp, Keyword::FirstStrike, &reg));
}

// -------------------------------------------------------------------------
// Auras that grant P/T plus a keyword or a restriction
// -------------------------------------------------------------------------

/// Bug #12: Spectral Flight should give +2/+2 AND flying.
#[test]
fn spectral_flight_gives_plus_two_and_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let sf = castable_spell(&mut state, &reg, "Spectral Flight", P0);

    state = cast_and_resolve(&state, &reg, sf, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(4),
        "Spectral Flight should give +2 power");
    assert_eq!(state.effective_toughness(creature, &reg), Some(4),
        "Spectral Flight should give +2 toughness");
    assert!(state.has_keyword(creature, Keyword::Flying, &reg),
        "Spectral Flight should grant flying");
}

/// Bug #13: Furor of the Bitten should give +2/+2 AND force attack.
#[test]
fn furor_of_the_bitten_gives_plus_two_and_forces_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 1, 1);
    let furor = castable_spell(&mut state, &reg, "Furor of the Bitten", P0);

    state = cast_and_resolve(&state, &reg, furor, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "Furor of the Bitten should give +2 power (1 + 2 = 3)");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Furor of the Bitten should give +2 toughness (1 + 2 = 3)");

    // The creature should be forced to attack.
    assert!(state.has_effect(creature, &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), &reg), "Furor of the Bitten should force creature to attack");
}

/// Bug #14: Bonds of Faith should give +2/+2 to Humans.
#[test]
fn bonds_of_faith_gives_plus_two_to_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elder Cathar is a Human.
    let creature = named_permanent(&mut state, &reg, "Elder Cathar", P0);

    let base_power = state.effective_power(creature, &reg).unwrap();
    let base_toughness = state.effective_toughness(creature, &reg).unwrap();

    let bof = castable_spell(&mut state, &reg, "Bonds of Faith", P0);
    state = cast_and_resolve(&state, &reg, bof, vec![Target::Object(creature)]);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.effective_power(creature, &reg), Some(base_power + 2),
        "Bonds of Faith should give +2 power to Human");
    assert_eq!(state.effective_toughness(creature, &reg), Some(base_toughness + 2),
        "Bonds of Faith should give +2 toughness to Human");
    assert!(state.can_attack(creature, &reg),
        "Human with Bonds of Faith should still be able to attack");
}

/// Bonds of Faith on a non-Human should prevent attack/block, NOT give +2/+2.
#[test]
fn bonds_of_faith_locks_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let bof = castable_spell(&mut state, &reg, "Bonds of Faith", P0);
    state = cast_and_resolve(&state, &reg, bof, vec![Target::Object(creature)]);
    triggers::process_triggers(&mut state, &reg);

    // Should NOT get +2/+2.
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "Non-Human should NOT get +2 power from Bonds of Faith");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3),
        "Non-Human should NOT get +2 toughness from Bonds of Faith");

    // Should be locked down.
    assert!(!state.can_attack(creature, &reg),
        "Non-Human with Bonds of Faith should not be able to attack");
    assert!(!state.can_block(creature, &reg),
        "Non-Human with Bonds of Faith should not be able to block");
}

// -------------------------------------------------------------------------
// Ranger's Guile
// -------------------------------------------------------------------------

/// "Target creature you control gets +1/+1 and gains hexproof until end of
/// turn." You control — so an opponent's creature is not offered, and one of
/// yours is. The second half matters: without it, an engine offering no
/// targets at all satisfies the first.
#[test]
fn rangers_guile_targets_only_your_own_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 2, 2);
    let theirs = ready_creature(&mut state, P1, 3, 3);
    let guile = castable_spell(&mut state, &reg, "Ranger's Guile", P0);

    let offered = offered_targets(&state, &reg, guile);
    assert!(offered.contains(&Target::Object(mine)), "your own creature; offered {offered:?}");
    assert!(!offered.contains(&Target::Object(theirs)), "not the opponent's");
}

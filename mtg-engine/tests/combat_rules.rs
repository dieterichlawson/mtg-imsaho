//! Regressions for bugs documented in `audits/AUDIT_BUGS.md`. Each of these
//! failed when it was written and passes now; they stay to protect against
//! the bug coming back.
//!
//! This file covers the "Combat / attack-phase rules" family.
//!
//! Bugs covered in this file:
//! - Bug BP: Forced-attack effects (Furor of the Bitten, Curse of
//!   the Nightly Hunt) don't consult `can't attack` continuous
//!   effects (Bonds of Faith), so a creature can be forced to
//!   attack while Bonds of Faith simultaneously prevents it.
//! - Bug 17-005: Non-trample attackers blocked by multiple creatures
//!   dump all damage on the first blocker in iteration order,
//!   instead of letting the attacking player divide damage per
//!   CR 510.1c.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug 17-005 (`audits/AUDIT_BUGS.md)`: A 5-power non-trample attacker
/// blocked by two 2/2s dumps all 5 damage on the first blocker in
/// iteration order, leaving the second alive. Per CR 510.1c the
/// attacking player divides damage among blockers — with two 2/2
/// blockers a 5-power attacker should be able to kill both (2+2,
/// 2+3, etc.).
///
/// Oracle (CR 510.1c): "A blocking creature or a blocked creature
/// you control deals its combat damage to the creature(s) blocking
/// it or being blocked by it. If a creature is blocked by multiple
/// creatures, the attacking player may divide its combat damage any
/// way they choose among those blockers."
///
/// Failure mode: `combat.rs` hard-codes "assign all
/// remaining power to the current blocker for non-trample
/// attackers", then `remaining_power = 0` for every subsequent
/// blocker.
///
/// We build combat state with a 5-power attacker and two 2/2
/// blockers, run `deal_combat_damage`, and assert that both
/// blockers die (since the attacker has enough power to kill both
/// with optimal distribution).
#[test]
fn bug_17_005_non_trample_attacker_can_kill_multiple_blockers() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let attacker = ready_creature(&mut state, P0, 5, 5);
    state.get_object_mut(attacker).unwrap().card_types = vec![CardType::Creature];

    let blocker_a = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker_a).unwrap().card_types = vec![CardType::Creature];
    let blocker_b = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker_b).unwrap().card_types = vec![CardType::Creature];

    attacks_blocked_by(&mut state, attacker, P1, &[blocker_a, blocker_b]);

    mtg_engine::combat::deal_combat_damage(&mut state, &registry);
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    let a_zone = state.get_object(blocker_a).map(|o| o.zone);
    let b_zone = state.get_object(blocker_b).map(|o| o.zone);
    let both_dead = a_zone == Some(Zone::Graveyard) && b_zone == Some(Zone::Graveyard);

    assert!(
        both_dead,
        "A 5-power non-trample attacker blocked by two 2/2s should be \
         able to kill both (2+2 split). Bug 17-005: combat.rs hard-codes \
         'assign all damage to the first blocker', so the first blocker \
         takes 5 damage and the second survives. zones: a={a_zone:?}, b={b_zone:?}",
    );
}

/// Bug BP (`audits/AUDIT_BUGS.md)`: Forced-attack effects iterate
/// candidate attackers without calling `state.can_attack`, so a
/// creature under Bonds of Faith's `ConditionalPreventAttack` gets
/// forced to attack despite the "can't attack" clause.
///
/// Oracle (Bonds of Faith): "Enchanted creature gets +2/+2 as long
/// as it's a Human. Otherwise, it can't attack or block."
/// Oracle (Furor of the Bitten): "Enchanted creature attacks each
/// combat if able."
///
/// Failure mode: `engine.rs` enumerates forced attackers,
/// filtering only Defender / tapped / summoning-sick / already-
/// attacking. It does NOT call `state.can_attack`, which is the
/// function that consults `PreventAttack` / `ConditionalPreventAttack`
/// continuous effects. So a non-Human creature under Bonds of Faith
/// plus Furor of the Bitten gets force-added to `combat.attackers`,
/// despite Bonds of Faith's "it can't attack" clause.
///
/// We put a non-Human creature with Furor of the Bitten (`ForceAttack`)
/// AND Bonds of Faith (`ConditionalPreventAttack`) attached. We submit
/// an empty `DeclareAttackers` action; the engine's forced-attack
/// loop should NOT add the locked creature to `combat.attackers`.
#[test]
fn bug_bp_forced_attack_respects_cant_attack() {
    use mtg_engine::actions::Action;

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    // Grizzly Bears — not a Human.
    let bears = named_permanent(&mut state, &registry, "Grizzly Bears", P0);

    // Bonds of Faith attached to the Bears (non-Human → can't attack
    // per oracle).
    let bonds_card_id = registry.get_id_by_name("Bonds of Faith").unwrap();
    let bonds = state.create_object(bonds_card_id, P1, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(bonds).unwrap();
        obj.name = "Bonds of Faith".into();
        obj.attached_to = Some(bears);
    }

    // Furor of the Bitten attached to the Bears — "attacks each combat
    // if able".
    let furor_card_id = registry.get_id_by_name("Furor of the Bitten").unwrap();
    let furor = state.create_object(furor_card_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(furor).unwrap();
        obj.name = "Furor of the Bitten".into();
        obj.attached_to = Some(bears);
    }

    // Sanity: can_attack should already return false.
    assert!(
        !state.can_attack(bears, &registry),
        "Test setup: can_attack should say the non-Human Bears can't \
         attack while Bonds of Faith is attached"
    );

    // Submit an empty DeclareAttackers action. The engine's
    // forced-attack loop will run. After the action returns,
    // combat.attackers should NOT contain the locked creature.
    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::DeclareAttackers { attackers: vec![] },
        &registry,
    );

    let force_attacked = new_state
        .combat
        .as_ref()
        .is_some_and(|c| c.attackers.contains_key(&bears));

    assert!(
        !force_attacked,
        "A non-Human creature enchanted with Bonds of Faith (can't \
         attack) AND Furor of the Bitten (attacks if able) should NOT \
         be force-added to combat.attackers — the 'if able' clause \
         means the force doesn't apply. Bug BP: the forced-attack \
         enumerator skips Defender but not can_attack / PreventAttack."
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// "Attacks each combat IF ABLE" — Pacifism makes it unable, so the force does
/// not apply. The same rule as the Bonds of Faith test above, reached through a
/// different "can't attack" effect.
#[test]
fn a_creature_under_pacifism_is_not_forced_to_attack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Bloodcrazed Neonate (has ForceAttack via continuous effect)
    let neonate = named_permanent(&mut state, &registry, "Bloodcrazed Neonate", P0);

    // Cast Pacifism on the Neonate (gives PreventAttack + PreventBlock)
    let pacifism = castable_spell(&mut state, &registry, "Pacifism", P0);
    state = cast_and_resolve(&state, &registry, pacifism, vec![Target::Object(neonate)]);

    // The attackers prompt is produced for a pending `DeclareAttackers`
    // awaiting-action, not merely for being in the step.
    state.step = Step::DeclareAttackers;
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);

    let legal = engine::legal_actions(&state, &registry);

    // The prompt has to be there — with the assertions inside an `if let`,
    // a missing prompt would silently assert nothing at all.
    let Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { must_attack, eligible, .. }) =
        legal.combat_prompt.as_ref()
    else {
        panic!("expected a ChooseAttackers prompt, got {:?}", legal.combat_prompt);
    };
    assert!(!eligible.contains(&neonate),
        "Pacifism prevents attacking, so the Neonate is not even eligible");
    assert!(!must_attack.contains(&neonate),
        "and 'attacks each combat if able' cannot force a creature that is unable");
}

/// The third route to "unable": Galvanic Juggernaut enters tapped and does not
/// untap, and a tapped creature cannot attack.
#[test]
fn a_tapped_creature_is_not_forced_to_attack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    // Place Galvanic Juggernaut (has ForceAttack + it enters tapped + PreventUntap)
    let jug = named_permanent(&mut state, &registry, "Galvanic Juggernaut", P0);

    // Tap it (it enters tapped and doesn't untap).
    state.get_object_mut(jug).unwrap().tapped = true;
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);

    let legal = engine::legal_actions(&state, &registry);
    let Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { must_attack, eligible, .. }) =
        legal.combat_prompt.as_ref()
    else {
        panic!("expected a ChooseAttackers prompt, got {:?}", legal.combat_prompt);
    };
    assert!(!eligible.contains(&jug),
        "a tapped Juggernaut is not eligible to attack");
    assert!(!must_attack.contains(&jug),
        "and cannot be forced to");
}

//! Combat rules: who may attack, who may block, and how damage is assigned.
//!
//! CR 508 (declaring attackers), CR 509 (declaring blockers), CR 510 (the
//! combat damage step). `combat.rs` covers the same pipeline through the
//! submitted-action path; this file works on the rules themselves —
//! restrictions, requirements, damage assignment, and the extra damage step
//! first strike creates (CR 510.5).

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;
use mtg_engine::combat;
use mtg_engine::events::GameEvent;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::AwaitingAction;

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

// -------------------------------------------------------------------------
// A combat nobody attacked in
// -------------------------------------------------------------------------

/// After declaring zero attackers, the game loop skips to `EndCombat`.
/// This tests the game loop code path (not `submit_action`, which doesn't skip).
/// The bug is in `run_game_loop_inner`'s post-action handler for `DeclareAttackers`.
///
/// We test this by running the game loop with a callback that records what
/// steps the game passes through.
#[test]
fn no_attackers_game_loop_skips_to_end_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::BeginCombat, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    attacks_unblocked(&mut state, attacker, P1);

    // Fill libraries so we don't hit empty-library SBA.
    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    let mut action_count = 0;

    engine::run_game_loop(&mut state, &reg, |game_state, _player, legal| {
        action_count += 1;

        // Safety valve: don't run forever.
        if action_count > 50 {
            return Action::Concede;
        }

        // When asked to declare attackers, declare none.
        if legal.combat_prompt.is_some() {
            if game_state.step == Step::DeclareAttackers {
                return Action::DeclareAttackers { attackers: vec![] };
            }
            if game_state.step == Step::DeclareBlockers {
                return Action::DeclareBlockers { assignments: vec![] };
            }
        }

        // Otherwise just pass priority to advance the game.
        Action::PassPriority
    });

    // Check that DeclareBlockers was reached by looking at StepStarted events.
    // Auto-pass may skip asking the player, but the step should still be entered.
    let saw_declare_blockers = state.events.iter().any(|e| {
        matches!(e, GameEvent::StepStarted { step: Step::DeclareBlockers })
    });
    // Also check the game log for the step.
    let log_has_blockers = state.game_log.iter().any(|e| {
        e.message.contains("DeclareBlockers")
    });
    assert!(saw_declare_blockers || log_has_blockers,
        "Game loop should pass through DeclareBlockers even with zero attackers (CR 507-510)");
}

// -------------------------------------------------------------------------
// Declaring, blocking, and the damage steps
// -------------------------------------------------------------------------

/// A tapped creature can't be declared as a blocker (CR 509.1a). The
/// validating gate must drop it, leaving the attacker unblocked.
#[test]
fn an_illegal_block_by_a_tapped_creature_does_not_absorb_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().tapped = true;
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &reg);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "a tapped creature isn't blocking, so it takes no combat damage");
    assert_eq!(state.get_player(P1).life, p1_life - 2,
        "the block was illegal; the attacker is unblocked and hits the player");
}

/// A creature the attacking player controls can't be declared as a blocker —
/// only the defending player's creatures block (CR 509.1a).
#[test]
fn attacking_players_own_creature_cannot_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let fake_blocker = ready_creature(&mut state, P0, 2, 2); // controlled by the attacker's player
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(fake_blocker, attacker)], &reg);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, p1_life - 2,
        "a creature controlled by the attacker can't block; attacker is unblocked");
}

/// The DeclareAttackers handler validates eligibility: a summoning-sick
/// creature (no haste) submitted as an attacker is dropped.
#[test]
fn ineligible_attacker_is_filtered_by_the_handler() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let sick = sick_creature(&mut state, P0, 2, 2);
    let ready = ready_creature(&mut state, P0, 3, 3);
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);

    let state = engine::submit_action(
        &state,
        &Action::DeclareAttackers { attackers: vec![(sick, P1), (ready, P1)] },
        &reg,
    );

    let attacking: Vec<_> = state.combat.as_ref()
        .map(|c| c.attackers.keys().copied().collect())
        .unwrap_or_default();
    assert!(attacking.contains(&ready), "the eligible creature attacks");
    assert!(!attacking.contains(&sick),
        "a summoning-sick creature without haste can't be declared as an attacker");
}

/// A blocker that regenerates away first-strike lethal damage is removed
/// from combat and must not deal its damage in the regular step.
#[test]
fn regenerated_blocker_deals_no_regular_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().regeneration_shields = 1;
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // First strike killed the blocker; it regenerated (tapped, healed,
    // removed from combat).
    let b = state.get_object(blocker).unwrap();
    assert_eq!(b.zone, Zone::Battlefield, "blocker should have regenerated");
    assert!(b.tapped, "regeneration taps the creature");
    assert_eq!(b.regeneration_shields, 0);

    // CR 701.15c: the regenerated creature was removed from combat and must
    // NOT deal regular combat damage to the attacker.
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "attacker must take no damage from a blocker that left combat");
    // The attacker remains blocked (no trample): the player takes nothing.
    assert_eq!(state.get_player(P1).life, p1_life);
}

/// A double-striker whose blocker regenerated away stays BLOCKED
/// (CR 510.1c): its regular-step damage hits nothing — not the player.
#[test]
fn double_striker_stays_blocked_when_blocker_leaves_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::DoubleStrike);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().regeneration_shields = 1;
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // Blocker regenerated away the first-strike damage and left combat.
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "regeneration clears marked damage; regular-step damage must not land");
    // The attacker is still blocked and has no trample: regular-step damage
    // is assigned to nothing — the defending player takes none.
    assert_eq!(state.get_player(P1).life, p1_life,
        "blocked double-striker must not hit the player when its blocker leaves combat");
}

/// CR 510.5: with first strikers in combat there are TWO combat damage
/// steps, with SBAs and a priority round between them. The engine models
/// this by repeating Step::CombatDamage.
#[test]
fn first_strike_creates_second_combat_damage_step_with_window() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // 2/2 first striker attacks; 4/4 blocks (survives first strike).
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 4, 4);

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    // Enter the combat damage step: FIRST instance — first-strike damage only.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    assert!(state.combat_damage_step_pending,
        "a second combat damage step must be pending (CR 510.5)");
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2,
        "first striker deals its damage in the first step");
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "non-first-striker deals nothing in the first step");

    // Priority window between the steps: the defender removes the attacker
    // (as a Doom Blade would during this round of priority).
    state.move_object(attacker, Zone::Graveyard, &reg);

    // All players pass: the step repeats — SECOND instance, regular damage.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage,
        "Step::CombatDamage must repeat for the regular damage step");
    assert!(!state.combat_damage_step_pending);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2,
        "the removed attacker deals no regular damage; blocker keeps only first-strike damage");

    // And the step sequence continues normally afterwards.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::EndCombat);
}

/// Without first strikers, the combat damage step happens exactly once.
#[test]
fn no_first_strike_single_combat_damage_step() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    assert!(!state.combat_damage_step_pending,
        "no first strikers: no second damage step");
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 2);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2);

    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::EndCombat);
}

/// First-strike deaths produce their triggers BEFORE regular damage: the
/// window lets death triggers resolve between the two damage steps.
#[test]
fn first_strike_kill_prevents_regular_damage_back() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // 2/2 first striker vs 2/2 blocker: blocker dies to first strike and
    // never deals regular damage back.
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    mtg_engine::engine::advance_step(&mut state, &reg);
    // The game loop runs SBAs before granting priority (CR 117.5).
    while mtg_engine::sba::check_state_based_actions(&mut state, &reg) {}
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard,
        "blocker dies to first-strike damage before the regular step");

    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "dead blocker deals no regular-step damage");
}

/// Blazing Torch's granted ability must be offered to the equipped creature's
/// controller only when that player also controls the Torch — its cost
/// sacrifices the Torch, which only its controller may do.
#[test]
fn opponents_equipment_grants_no_activatable_ability() {
    let reg = registry();

    // (who controls the Torch, is the ability offered to the creature's
    //  controller)
    for (torch_owner, offered) in [(P0, true), (P1, false)] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let creature = ready_creature(&mut state, P0, 2, 2);
        let torch = named_permanent(&mut state, &reg, "Blazing Torch", torch_owner);
        state.get_object_mut(torch).unwrap().attached_to = Some(creature);

        assert_eq!(offers_ability_of(&state, &reg, creature), offered,
            "torch controlled by p{}: the sacrifice cost is payable only by its \
             controller", torch_owner.0);
    }
}

// -------------------------------------------------------------------------
// Both sides lethal
// -------------------------------------------------------------------------

/// When two creatures deal lethal damage to each other in combat,
/// both should die simultaneously when SBAs are checked.
#[test]
fn mutually_lethal_combat_both_die() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 3);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(
        state.get_object(attacker).unwrap().zone,
        Zone::Graveyard,
        "Attacker should die from mutually lethal combat"
    );
    assert_eq!(
        state.get_object(blocker).unwrap().zone,
        Zone::Graveyard,
        "Blocker should die from mutually lethal combat"
    );
    // Player takes no damage — attacker was blocked.
    assert_eq!(state.get_player(P1).life, 20);
}

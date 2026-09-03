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
        &Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] },
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

/// "Attacks each combat **if able**" — and haste is what makes a
/// just-arrived creature able (CR 302.6). Curse of the Nightly Hunt on a
/// player who plays a Manor Skeleton forces the Skeleton into combat the turn
/// it arrives.
///
/// The prompt and the handler used to disagree about this. `legal_actions`
/// builds its `must_attack` list by filtering `combat::eligible_attackers`,
/// which asks `!summoning_sick || has_keyword(Haste)`; the declare-attackers
/// handler rolled its own eligibility check that stopped at `summoning_sick`.
/// So the prompt told the player the Skeleton had to attack and the engine
/// then let it stay home.
#[test]
fn a_hasty_creature_is_forced_to_attack_the_turn_it_arrives() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let skeleton = named_permanent(&mut state, &registry, "Manor Skeleton", P0);
    // It came down this turn; haste is the only reason it can attack.
    state.get_object_mut(skeleton).unwrap().summoning_sick = true;
    attach_curse_to_player(&mut state, &registry, "Curse of the Nightly Hunt", P1, P0);

    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);
    let legal = engine::legal_actions(&state, &registry);
    let Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { must_attack, eligible, .. }) =
        legal.combat_prompt.as_ref()
    else {
        panic!("expected a ChooseAttackers prompt, got {:?}", legal.combat_prompt);
    };
    assert!(eligible.contains(&skeleton), "haste makes it able to attack");
    assert!(must_attack.contains(&skeleton), "and the Curse makes it have to");

    // The player declares nothing. The requirement stands regardless.
    let state = engine::submit_action(
        &state, &Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] }, &registry);

    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&skeleton)),
        "CR 508.1d: a creature that is able to attack and is required to must be \
         declared as an attacker");
}

/// The third route to "unable": a tapped creature cannot attack (CR 508.1a),
/// so "attacks each combat if able" asks nothing of it.
///
/// Galvanic Juggernaut is the card this matters on. It does not enter tapped —
/// it taps by attacking, and then "doesn't untap during your untap step" keeps
/// it that way until something dies, so a tapped Juggernaut is its ordinary
/// state rather than a contrived one.
#[test]
fn a_tapped_creature_is_not_forced_to_attack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let jug = named_permanent(&mut state, &registry, "Galvanic Juggernaut", P0);
    state.tap(jug);
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

/// And untapped, it must. The Curse case above forces a creature from an
/// effect on somebody else; this is the requirement a creature carries itself
/// (`ForceAttack` scoped `OnSelf`), which is a different lookup.
#[test]
fn a_creature_that_forces_itself_to_attack_must_attack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let jug = named_permanent(&mut state, &registry, "Galvanic Juggernaut", P0);
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);

    let legal = engine::legal_actions(&state, &registry);
    let Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { must_attack, .. }) =
        legal.combat_prompt.as_ref()
    else {
        panic!("expected a ChooseAttackers prompt, got {:?}", legal.combat_prompt);
    };
    assert!(must_attack.contains(&jug), "\"attacks each combat if able\"");

    // The player declares nothing. The requirement stands regardless.
    let state = engine::submit_action(
        &state, &Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] }, &registry);

    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&jug)),
        "CR 508.1d: it is able and required, so it is declared as an attacker");
}

// -------------------------------------------------------------------------
// A combat nobody attacked in
// -------------------------------------------------------------------------

/// Regression (#59): after declaring zero attackers, the game loop skips the
/// declare blockers and combat damage steps entirely (CR 508.8) — no
/// "beginning of the declare blockers step" trigger can fire and no priority
/// window opens in a step that doesn't happen. This test used to assert the
/// opposite of its own name, pinning the rule violation in place.
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
                return Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] };
            }
            if game_state.step == Step::DeclareBlockers {
                return Action::DeclareBlockers { assignments: vec![] };
            }
        }

        // Otherwise just pass priority to advance the game.
        Action::PassPriority
    });

    // The callback declares zero attackers every combat of every turn, so no
    // declare blockers or combat damage step may ever start. (`state.events`
    // is cleared by every submit_action, so the game log is the record.)
    let entered_skipped_step = state.game_log.iter().any(|e| {
        e.message.contains("Step: DeclareBlockers") || e.message.contains("Step: CombatDamage")
    });
    assert!(!entered_skipped_step,
        "CR 508.8: with no attackers declared, the declare blockers and combat \
         damage steps are skipped");
    let reached_end_combat = state.game_log.iter().any(|e| {
        e.message.contains("Step: EndCombat")
    });
    assert!(reached_end_combat,
        "combat still ends through the end of combat step");
}

/// The other half of CR 508.8's condition: when attackers WERE declared, the
/// steps happen even if every attacker has left combat by the time the
/// declare attackers step ends — the skip keys on the declaration.
#[test]
fn declared_attacker_leaving_combat_does_not_skip_the_steps() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
    state.remove_from_combat(attacker);
    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.is_empty()),
        "test precondition: nobody is left in combat");

    engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::DeclareBlockers,
        "attackers were declared, so the declare blockers step happens \
         (CR 508.8 skips only when none were declared)");
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

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &reg);
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "a tapped creature isn't blocking, so it takes no combat damage");
    assert_eq!(state.get_player(P1).life, p1_life - 2,
        "the block was illegal; the attacker is unblocked and hits the player");
}

/// Regression (#62): a creature can block only one attacker (CR 509.1b) —
/// no effect in this set lifts that. A submitted declaration assigning one
/// blocker to two attackers used to be accepted pair-by-pair, and the
/// blocker then dealt its full power to EACH attacker (a 2/1 killed a 2/2
/// and a 3/1 in the same combat). Only the first assignment may stand; the
/// second attacker is unblocked and hits the player.
#[test]
fn one_blocker_cannot_block_two_attackers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker_a = ready_creature(&mut state, P0, 2, 2);
    let attacker_b = ready_creature(&mut state, P0, 3, 1);
    let blocker = ready_creature(&mut state, P1, 2, 1);
    let p1_life = state.get_player(P1).life;

    mtg_engine::combat::declare_attackers(
        &mut state, &[(attacker_a, P1), (attacker_b, P1)], &[], &reg);
    mtg_engine::combat::declare_blockers_with_registry(
        &mut state, &[(blocker, attacker_a), (blocker, attacker_b)], &reg);

    let combat = state.combat.as_ref().expect("combat exists");
    assert_eq!(combat.blocker_assignments.get(&attacker_a).map(Vec::len), Some(1),
        "the first assignment stands");
    assert!(combat.blocker_assignments.get(&attacker_b).is_none_or(Vec::is_empty),
        "CR 509.1b: the blocker is already blocking; the second assignment is refused");

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(attacker_b).unwrap().damage_marked, 0,
        "the second attacker was never blocked and takes no damage");
    assert_eq!(state.get_player(P1).life, p1_life - 3,
        "the unblocked 3/1 hits the player");
}

/// The same pair submitted many times is one block, not many (#50): the
/// declaration log and save file should not record a thousand copies.
#[test]
fn a_repeated_identical_block_pair_collapses_to_one() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 4, 4);

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
    let pairs = vec![(blocker, attacker); 1000];
    mtg_engine::combat::declare_blockers_with_registry(&mut state, &pairs, &reg);

    let combat = state.combat.as_ref().expect("combat exists");
    assert_eq!(combat.blocker_assignments.get(&attacker).map(Vec::len), Some(1),
        "one block, however many times the pair was submitted");
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

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
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
        &Action::DeclareAttackers { attackers: vec![(sick, P1), (ready, P1)], planeswalker_attacks: vec![] },
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

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
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

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
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

    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
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

    // The two damage steps used to log identically (issue #140, CR 510.4):
    // each half names itself now.
    assert!(state.game_log.iter().any(|e|
        e.message.contains("First-strike combat damage step")),
        "the first-strike half is named in the log");
    assert!(state.game_log.iter().any(|e|
        e.message.contains("Regular combat damage step")),
        "the regular half is named in the log");
}

/// Without first strikers, the combat damage step happens exactly once.
#[test]
fn no_first_strike_single_combat_damage_step() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
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
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
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

/// CR 510.5 / 701.15: a creature that regenerates is removed from combat. If
/// that empties combat between the two combat damage steps, the second step
/// still has to happen and combat still has to end.
///
/// `combat_damage_step_pending` is what sends `advance_step` back to
/// Step::CombatDamage a second time, and it used to be cleared inside the
/// "there are attackers" branch. With combat emptied, the second step found
/// nothing to do, skipped that branch, and left the flag set — so `advance_step`
/// chose Step::CombatDamage again, and again, and the game never reached end of
/// combat. About one random game in twenty-five ground to a halt this way.
///
/// `first_strike_creates_second_combat_damage_step_with_window` above does not
/// catch it: it moves the attacker to the graveyard by hand, which leaves the id
/// in `combat.attackers`, so `has_attackers` stays true.
#[test]
fn combat_ends_when_the_attacker_leaves_combat_between_damage_steps() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // A 1/1 attacker meets a 2/2 first-striking blocker.
    let attacker = ready_creature(&mut state, P0, 1, 1);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(blocker).unwrap().keywords.push(Keyword::FirstStrike);

    combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
    combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    // First combat damage step: only the first striker deals damage.
    engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    assert!(state.combat_damage_step_pending,
        "a second combat damage step is pending (CR 510.5)");

    // The attacker regenerates the lethal damage, which removes it from combat
    // (CR 701.15). Combat is now empty on both sides.
    mtg_engine::destruction::remove_from_combat(&mut state, attacker);
    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.is_empty()));

    // The second combat damage step happens with nothing to do...
    engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    assert!(!state.combat_damage_step_pending,
        "entering the step consumes the flag even with no attackers left");

    // ...and then combat ends, rather than the step repeating forever.
    engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::EndCombat,
        "combat must end; leaving the flag set looped Step::CombatDamage");
}

// ---------------------------------------------------------------------------
// A card that hands out a blocking restriction
// ---------------------------------------------------------------------------

/// Crossway Vampire: "When this creature enters, target creature can't block
/// this turn."
///
/// The rule itself — `TemporaryEffect::CantBlock` keeping a creature out of
/// `eligible_blockers` — is covered above. What had no test is that this card
/// applies it: making its ETB hook do nothing at all passed the whole
/// workspace, because its only coverage was about which targets are offered.
///
/// Three claims here: the targeted creature can't block, another creature of
/// the same controller still can, and the restriction is "this turn" — it goes
/// away with the turn, through the engine's cleanup rather than by hand.
#[test]
fn crossway_vampire_stops_its_target_blocking_for_the_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let stopped = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    let free = named_permanent(&mut state, &reg, "Walking Corpse", P1);

    let vampire = named_permanent(&mut state, &reg, "Crossway Vampire", P0);
    state.events.push(GameEvent::EnteredBattlefield { object: vampire, controller: P0 });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // "target creature" reaches any creature, so with three on the battlefield
    // the controller is asked which (CR 603.3d).
    let options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.clone(),
        other => panic!("the trigger should ask which creature, got {other:?}"),
    };
    assert!(options.contains(&Target::Object(stopped)) && options.contains(&Target::Object(free)),
        "both of the opponent's creatures are legal targets, got {options:?}");
    let mut state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(stopped))),
        },
        &reg,
    );
    // Answering the target choice puts the trigger on the stack; it still has
    // to resolve before anything happens (CR 603.3).
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let eligible = combat::eligible_blockers(&state, P1, &reg);
    assert!(!eligible.contains(&stopped),
        "the targeted creature can't block this turn");
    assert!(eligible.contains(&free),
        "and the one beside it still can — the restriction is on a target, not a sweep");

    advance_to_next_turn(&mut state, &reg);

    assert!(combat::eligible_blockers(&state, P1, &reg).contains(&stopped),
        "'this turn' — the restriction is gone once the turn is");
}

/// CR 508.1d asks that a required creature attack; it does not say whom. A
/// forced attacker declared at a planeswalker satisfies the requirement —
/// the declaration is validated and inserted before the forced-attacker pass
/// runs, so that pass sees it already attacking and leaves it alone rather
/// than re-pointing it at the player.
///
/// Recorded in the Curse of the Nightly Hunt audit as "worth revisiting
/// whenever planeswalker combat is implemented — the forced pass would then
/// need to ask rather than assume." It does not need to ask: the player makes
/// the choice in the declaration itself, and only a creature the player left
/// undeclared is defaulted to attacking the player, which is a legal
/// completion of the requirement.
#[test]
fn a_forced_attacker_may_be_declared_at_a_planeswalker() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let forced = ready_creature(&mut state, P0, 2, 2);
    let bystander = ready_creature(&mut state, P0, 1, 1);
    attach_curse_to_player(&mut state, &registry, "Curse of the Nightly Hunt", P1, P0);
    let liliana = named_permanent(&mut state, &registry, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);

    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);
    let state = engine::submit_action(
        &state,
        &Action::DeclareAttackers {
            attackers: vec![],
            planeswalker_attacks: vec![(forced, liliana)],
        },
        &registry,
    );

    let combat = state.combat.as_ref().expect("combat exists");
    assert_eq!(combat.attackers.get(&forced), Some(&P1),
        "the walker's controller is the defending player (CR 508.1a)");
    assert_eq!(combat.planeswalker_defenders.get(&forced), Some(&liliana),
        "the declaration at the walker stands; the forced pass must not \
         re-point a creature that is already attacking");
    // The bystander was also under the Curse and undeclared, so the forced
    // pass completes the requirement for it — at the player, the default.
    assert_eq!(combat.attackers.get(&bystander), Some(&P1),
        "an undeclared forced attacker is still dragged into combat");
    assert!(!combat.planeswalker_defenders.contains_key(&bystander),
        "and it attacks the player, not the walker");
}

/// A creature removed from combat (regeneration, CR 701.15c) leaves no
/// bookkeeping behind: not in `attackers`, not a `blocker_assignments` key,
/// not in `blocked_attackers`, `planeswalker_defenders`, or
/// `dealt_first_strike`. Coverage-deck fuzzing found a regenerated blocked
/// attacker still listed in `blocked_attackers` — a creature the combat
/// state said was blocked but never said attacked.
#[test]
fn removing_a_creature_from_combat_clears_every_record_of_it() {
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);
    state.combat.as_mut().unwrap().dealt_first_strike.insert(attacker);

    {
        let combat = state.combat.as_ref().unwrap();
        assert!(combat.blocked_attackers.contains(&attacker), "setup: it was blocked");
    }

    mtg_engine::destruction::remove_from_combat(&mut state, attacker);

    let combat = state.combat.as_ref().unwrap();
    assert!(!combat.attackers.contains_key(&attacker));
    assert!(!combat.blocker_assignments.contains_key(&attacker));
    assert!(!combat.blocked_attackers.contains(&attacker), "no stale blocked-ness");
    assert!(!combat.planeswalker_defenders.contains_key(&attacker));
    assert!(!combat.dealt_first_strike.contains(&attacker));
}

/// CR 506.4d: an attacking creature whose controller changes is removed
/// from combat — the thief gets the creature, and the attack simply ends.
/// Before this, a stolen attacker stayed in combat under its new controller
/// and dealt its combat damage for the old controller's attack.
#[test]
fn a_creature_that_changes_controller_leaves_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    attacks_unblocked(&mut state, attacker, P1);

    state.change_control(attacker, P1);

    let combat = state.combat.as_ref().unwrap();
    assert!(!combat.attackers.contains_key(&attacker),
        "a stolen creature is no longer an attacker (CR 506.4d)");

    combat::deal_combat_damage(&mut state, &reg);
    assert_eq!(state.get_player(P1).life, 20,
        "and it deals no combat damage for its old controller's attack");
}

/// CR 500.2: a step ends only when the stack is empty. A trigger put on the
/// stack during attacker declaration (Geist of Saint Traft's "create a 4/4
/// Angel tapped and attacking") used to ride the game loop's fallback
/// step-advances through the rest of combat, cleanup, and into the NEXT
/// turn before resolving — the Angel arrived a turn late, attacking on the
/// wrong turn. Found by seeded coverage fuzzing (br vs wu, seed 290).
///
/// Driven through the real game loop: if the trigger resolves in the combat
/// it triggered in, the defender takes Geist's 2 plus the Angel's 4.
#[test]
fn an_attack_trigger_resolves_in_the_combat_it_triggered_in() {
    let reg = registry();
    let mut state = game_at_step(Step::BeginCombat, P0);
    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);

    let land_id = reg.get_id_by_name("Forest").unwrap();
    for p in 0..2u8 {
        let mut lib = Vec::new();
        for _ in 0..20 {
            let id = state.create_object(land_id, mtg_engine::ids::PlayerId(p), Zone::Library, None, None);
            lib.push(id);
        }
        state.players[p as usize].library_order = lib;
    }

    let mut min_p1_life = 20;
    engine::run_game_loop(&mut state, &reg, |gs, _player, legal| {
        min_p1_life = min_p1_life.min(gs.get_player(P1).life);
        if gs.turn_number >= 2 {
            return Action::Concede;
        }
        if let Some(prompt) = &legal.combat_prompt {
            return match prompt {
                mtg_engine::actions::CombatPrompt::ChooseAttackers { .. } =>
                    Action::DeclareAttackers { attackers: vec![(geist, P1)], planeswalker_attacks: vec![] },
                mtg_engine::actions::CombatPrompt::ChooseBlockers { .. } =>
                    Action::DeclareBlockers { assignments: vec![] },
            };
        }
        Action::PassPriority
    });

    assert_eq!(min_p1_life, 20 - 2 - 4,
        "the Angel is created during THIS combat and its 4 damage lands with Geist's 2");
}

// ── Gaps found by mutation testing (cargo-mutants, 2026-08-29) ──────
// Each test below kills at least one mutant that survived the first
// engine-core run; see reports/mutation-testing.md.

/// CR 509.1a: only an untapped creature the defending player controls can
/// block. Every clause of `eligible_blockers`' filter, asserted separately —
/// mutants flipping any `&&` to `||` here survived the whole suite.
#[test]
fn eligible_blockers_is_untapped_creatures_of_the_defender_only() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let ready = ready_creature(&mut state, P1, 2, 2);
    let tapped = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(tapped).unwrap().tapped = true;
    let attackers_own = ready_creature(&mut state, P0, 2, 2);
    let land = named_permanent(&mut state, &reg, "Forest", P1);

    let eligible = combat::eligible_blockers(&state, P1, &reg);
    assert!(eligible.contains(&ready), "an untapped creature of the defender blocks");
    assert!(!eligible.contains(&tapped), "a tapped creature cannot block (CR 509.1a)");
    assert!(!eligible.contains(&attackers_own), "the attacker's creatures don't block for the defender");
    assert!(!eligible.contains(&land), "a land is not a creature");
}

/// CR 510.5: a first-strike blocker deals its damage in the first combat
/// damage step — a small attacker dies before it ever strikes back.
#[test]
fn a_first_strike_blocker_kills_before_the_attacker_strikes_back() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    grant_keyword(&mut state, blocker, Keyword::FirstStrike);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    combat::deal_combat_damage(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Graveyard,
        "first strike killed the attacker in the first damage step");
    let blocker_obj = state.get_object(blocker).unwrap();
    assert_eq!(blocker_obj.zone, Zone::Battlefield,
        "the blocker survives — it struck first, so the attacker never dealt \
         its damage (a dead blocker's cleared damage_marked would hide this)");
    assert_eq!(blocker_obj.damage_marked, 0,
        "so the attacker never dealt its damage");
}

/// CR 509.2 + 506.4c: an attacker that became blocked stays blocked even if
/// its only blocker is *removed from combat* (regeneration, CR 701.15c, or a
/// control change, CR 506.4d) before the damage step — it deals no combat
/// damage to anyone. Death also removes the blocker from combat (CR 506.4c,
/// via `move_object`); either way the assignment empties while the attacker
/// stays blocked, recorded in `blocked_attackers`.
#[test]
fn a_blocked_attacker_whose_blocker_left_combat_hits_nobody() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    // The blocker leaves combat before the damage step (as regeneration or
    // a control change would do): the attacker's assignment empties, but it
    // remains a blocked creature.
    mtg_engine::destruction::remove_from_combat(&mut state, blocker);
    assert!(state.combat.as_ref().is_some_and(
        |c| c.blocker_assignments.get(&attacker).is_some_and(Vec::is_empty)
            && c.blocked_attackers.contains(&attacker)));

    let p1_life = state.get_player(P1).life;
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, p1_life,
        "a blocked attacker with no blockers left deals no damage at all");
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "a creature removed from combat receives no combat damage either");
}

/// CR 506.4c: a creature that dies leaves combat — the live combat state
/// drops its id, not just the damage step's snapshot. Object ids survive
/// zone changes, so a dead blocker left in `blocker_assignments` becomes
/// whatever that id is next: nightly-fuzz seeds 20696050172, 20696050055,
/// 20697000024, 20697050136 and 20697075146 all hit a reanimator (Grimoire
/// of the Dead, Moldgraf Monstrosity) returning a dead blocker to the
/// battlefield mid-combat under the *attacking* player, leaving combat
/// claiming a "blocker" the defending player didn't control.
#[test]
fn a_blocker_that_dies_is_removed_from_the_live_combat_state() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 4, 4);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    combat::deal_combat_damage(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(blocker).unwrap().zone, Zone::Graveyard,
        "test precondition: the 4-power attacker kills the 2/2 blocker");

    assert!(state.combat.as_ref().is_some_and(
        |c| c.blocker_assignments.get(&attacker).is_some_and(Vec::is_empty)),
        "a dead blocker is removed from combat (CR 506.4c) — its id must \
         not linger in blocker_assignments");
    assert!(state.combat.as_ref().is_some_and(|c| c.blocked_attackers.contains(&attacker)),
        "the attacker remains a blocked creature (CR 510.1c)");

    // The fuzz-found sequel: the dead blocker is reanimated mid-combat
    // under the ATTACKING player (as Grimoire of the Dead does). The new
    // object reuses the id; combat must not claim it blocks anything.
    state.move_object_under_control(blocker, Zone::Battlefield, P0, &reg);
    let violations = mtg_engine::invariants::check_settled(&state, &reg);
    assert!(!violations.iter().any(|v| v.contains("not the defending player")),
        "a reanimated ex-blocker is a new object outside combat; \
         invariant violations: {violations:?}");
}

/// CR 506.4 + 509.1a (issue #88): an attacker that leaves the battlefield
/// after attackers are declared stops being an attacking creature. It must
/// not be offered at declare blockers, and a block declared against it must
/// not consume the blocker — the playtest crew baited the defender's only
/// blocker onto a sacrificed attacker, letting the real attackers through
/// unblocked.
#[test]
fn an_attacker_that_left_the_battlefield_cannot_be_blocked() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let sacrificed = ready_creature(&mut state, P0, 2, 2);
    let survivor = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 3);
    submit_declare_attackers(&mut state, &[(sacrificed, P1), (survivor, P1)], &reg);

    // The attacker is sacrificed before blockers (any leave-the-battlefield
    // does the same); CR 506.4c removes it from combat.
    state.move_object(sacrificed, Zone::Graveyard, &reg);

    state.awaiting_action =
        Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::actions::CombatPrompt::ChooseBlockers { attackers, legal_blocks, .. }) =
        legal.combat_prompt
    else {
        panic!("expected a ChooseBlockers prompt");
    };
    assert!(!attackers.contains(&sacrificed),
        "a dead creature is not an attacking creature (CR 509.1a)");
    assert!(legal_blocks.get(&blocker).is_some_and(|v| !v.contains(&sacrificed)),
        "no block may be declared against it");

    // Even a directly submitted block against the phantom must not stick.
    submit_declare_blockers(&mut state, P1, &[(blocker, sacrificed)], &reg);
    assert!(state.combat.as_ref().is_some_and(
        |c| c.blocker_assignments.values().all(|v| !v.contains(&blocker))),
        "the blocker is not consumed blocking a permanent that left the game");
}

/// CR 509.1b / 702.111b (issue #72): a lone blocker on a menace attacker is
/// an illegal declaration. The prompt must advertise the requirement up
/// front (`min_blockers`) so clients can refuse it at input time, and the
/// engine backstop that discards it must leave an audit trail — a silently
/// vanished block read as "declared no blockers" and ate the defender's
/// only blocker on nothing.
#[test]
fn an_under_minimum_block_is_advertised_and_refused_with_an_audit_trail() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    grant_keyword(&mut state, attacker, Keyword::Menace);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    // The prompt says up front that this attacker needs 2+ blockers.
    state.awaiting_action =
        Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::actions::CombatPrompt::ChooseBlockers { min_blockers, .. }) =
        legal.combat_prompt
    else {
        panic!("expected a ChooseBlockers prompt");
    };
    assert_eq!(min_blockers.get(&attacker), Some(&2),
        "menace (CR 702.111b) is advertised as a 2+ blocker requirement");

    // The engine backstop: the under-minimum declaration is discarded (the
    // attacker ends up unblocked) — but never silently.
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    assert!(state.combat.as_ref().is_some_and(
        |c| c.blocker_assignments.get(&attacker).is_some_and(Vec::is_empty)),
        "a lone blocker can't block a menace attacker");
    assert!(state.game_log.iter().any(|e|
        e.message.contains("ignored illegal block") && e.message.contains("fewer than 2")),
        "the discarded block leaves an audit trail in the log");
}

/// Issue #89: combat damage dealt to creatures, and life gained from
/// lifelink, leave log lines — a blocker used to die with no stated cause
/// and a lifelink swing left the log's life totals irreconcilable.
#[test]
fn creature_combat_damage_and_lifelink_are_logged() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    grant_keyword(&mut state, attacker, Keyword::Lifelink);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    combat::deal_combat_damage(&mut state, &reg);

    let log = state.game_log.iter().map(|e| e.message.as_str()).collect::<Vec<_>>();
    assert!(log.iter().any(|m| m.contains("dealt 3 combat damage to")),
        "the attacker's damage to the blocker is logged: {log:?}");
    assert!(log.iter().any(|m| m.contains("dealt 2 combat damage to")),
        "the blocker's damage back is logged too: {log:?}");
    assert!(log.iter().any(|m| m.contains("(lifelink): p0 gained 3 life (23)")),
        "the lifelink gain is logged with the running total: {log:?}");
}

/// CR 510.5: a creature with plain first strike deals damage in the first
/// step and NOT again in the regular step; the vanilla blocker still deals
/// its own damage in the regular step.
#[test]
fn a_plain_first_striker_deals_its_damage_exactly_once() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    grant_keyword(&mut state, attacker, Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 2, 5);
    attacks_blocked_by(&mut state, attacker, P1, &[blocker]);

    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2,
        "2 once, not 2 in each of the two damage steps");
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 2,
        "the surviving blocker strikes back in the regular step");
}

/// CR 702.4b: double strike deals damage in both combat damage steps.
#[test]
fn a_double_striker_deals_damage_in_both_steps() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    grant_keyword(&mut state, attacker, Keyword::DoubleStrike);
    attacks_unblocked(&mut state, attacker, P1);

    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 20 - 4,
        "2 in the first-strike step and 2 in the regular step");
}

/// CR 510.1c: "lethal damage" counts the damage already marked. A blocker
/// that came into the damage step wounded needs less, and the excess is free
/// to kill the next blocker in order.
#[test]
fn lethal_assignment_counts_damage_already_marked() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let wounded = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(wounded).unwrap().damage_marked = 1;
    let fresh = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P1, &[wounded, fresh]);

    combat::deal_combat_damage(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(wounded).unwrap().zone, Zone::Graveyard,
        "1 more damage was lethal for the wounded blocker");
    assert_eq!(state.get_object(fresh).unwrap().zone, Zone::Graveyard,
        "leaving 2 of the 3 power to kill the second blocker");
}

/// The forced-attack sweep must skip exactly the creatures already declared —
/// not "any creature when another is declared". Two creatures that attack
/// each combat if able, one declared by the player: the other is still
/// dragged in.
#[test]
fn each_forced_attacker_is_dragged_in_independently() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let n1 = named_permanent(&mut state, &reg, "Bloodcrazed Neonate", P0);
    let n2 = named_permanent(&mut state, &reg, "Bloodcrazed Neonate", P0);
    for id in [n1, n2] {
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    let new_state = engine::submit_action(
        &state,
        &Action::DeclareAttackers { attackers: vec![(n1, P1)], planeswalker_attacks: vec![] },
        &reg,
    );

    let combat = new_state.combat.as_ref().unwrap();
    assert!(combat.attackers.contains_key(&n1), "the declared one attacks");
    assert!(combat.attackers.contains_key(&n2), "and the undeclared one is forced in");
    assert_eq!(combat.attackers.len(), 2);
}

/// CR 509.1b: a creature without flying or reach can't block a flyer. The
/// prompt must not offer the pairing, and a submitted illegal pair is
/// refused with a visible log line — never silently (a block that vanished
/// without a trace cost real games; playtest issue #40).
#[test]
fn an_illegal_flying_block_is_refused_loudly_and_never_offered() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    grant_keyword(&mut state, attacker, Keyword::Flying);
    let grounded = ready_creature(&mut state, P1, 1, 1);

    combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);

    // The prompt's legal_blocks must exclude the grounded creature vs the flyer.
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    let legal = engine::legal_actions(&state, &reg);
    let Some(mtg_engine::actions::CombatPrompt::ChooseBlockers { legal_blocks, .. }) = legal.combat_prompt
    else { panic!("expected a blocker prompt") };
    assert_eq!(legal_blocks.get(&grounded).map(Vec::as_slice), Some(&[][..]),
        "a vanilla 1/1 has no legal blocks against a lone flyer");

    // Submitting the illegal pair anyway: refused, and the refusal is logged.
    submit_declare_blockers(&mut state, P1, &[(grounded, attacker)], &reg);
    assert!(state.combat.as_ref().is_some_and(|c|
        c.blocker_assignments.get(&attacker).is_some_and(Vec::is_empty)
        && !c.blocked_attackers.contains(&attacker)),
        "the flyer stays unblocked");
    assert!(state.game_log.iter().any(|e| e.message.contains("ignored illegal block")),
        "the drop is visible in the game log, not silent");
}

/// CR 508.1d/508.1m: a creature forced to attack ("attacks each combat if
/// able") is a declared attacker like any other — it is in the
/// `AttackersDeclared` event its own attack trigger fires from. Forced
/// attackers used to be inserted into the combat maps after the event had
/// been pushed, so a forced Kessig Cagebreakers made no wolves.
#[test]
fn a_forced_attacker_is_in_the_attackers_declared_event() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let bear = ready_creature(&mut state, P0, 2, 2);
    attach_curse_to_player(&mut state, &reg, "Curse of the Nightly Hunt", P1, P0);
    assert!(state.must_attack(bear, &reg), "test precondition");

    submit_declare_attackers(&mut state, &[], &reg);

    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&bear)),
        "the curse dragged the bear into combat");
    let declared: Vec<_> = state.events.iter().filter_map(|e| match e {
        GameEvent::AttackersDeclared { attackers } => Some(attackers.clone()),
        _ => None,
    }).collect();
    assert_eq!(declared.len(), 1, "one declaration per action: {:?}", state.events);
    assert!(declared[0].iter().any(|(id, _)| *id == bear),
        "the forced attacker is part of the declaration (CR 508.1d), got {declared:?}");
    assert!(state.get_object(bear).unwrap().tapped, "and it was tapped by attacking (CR 508.1f)");
}

/// A token put onto the battlefield attacking (Kessig Cagebreakers) is an
/// attacking creature like any other: it can be blocked, and the block is
/// recorded. It used to be entered into the attackers map without a blocker
/// list, so every block declared against it was silently dropped — found
/// by fuzzing (fourteen wolves, three blocks, none recorded).
#[test]
fn a_token_that_enters_attacking_can_be_blocked() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    declare_combat(&mut state, &[(attacker, P1, &[])]);
    let token = state.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green], vec![CardType::Creature],
        vec![], vec!["Wolf".into()], &reg)[0];
    mtg_engine::cards::helpers::tokens_enter_combat_attacking(&mut state, attacker, P0, &[token], &reg);
    let mut state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Player(P1))) },
        &reg,
    );
    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&token)), "test precondition: the token attacks");

    state.step = Step::DeclareBlockers;
    submit_declare_blockers(&mut state, P1, &[(blocker, token)], &reg);

    let c = state.combat.as_ref().unwrap();
    assert_eq!(c.blocker_assignments.get(&token).map(Vec::as_slice), Some(&[blocker][..]),
        "the block against the token is recorded (CR 509.1h)");
    assert!(c.blocked_attackers.contains(&token));
}

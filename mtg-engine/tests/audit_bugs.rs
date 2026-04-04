//! Failing tests that demonstrate bugs found by the Sonnet 4.6 audit.
//! Each test documents a specific issue and is expected to FAIL until the bug is fixed.

mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

// ═══════════════════════════════════════════════════════════════
// ENGINE: SUMMONING SICKNESS
// Tap abilities should not be activatable on the turn a creature enters.
// ═══════════════════════════════════════════════════════════════

/// Bug: Avacynian Priest can activate {1}, {T} ability on the turn it enters.
/// The engine checks `requires_tap && obj_tapped` (line 356) but never checks
/// `summoning_sick`. Per MTG rules, creatures with summoning sickness cannot
/// use abilities with {T} in the cost.
#[test]
fn bug_summoning_sickness_not_enforced_for_tap_abilities() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Avacynian Priest with summoning sickness (just entered this turn)
    let priest = {
        let card_id = registry.get_id_by_name("Avacynian Priest").unwrap();
        let data = registry.card_data(card_id).unwrap();
        let id = state.create_object(card_id, P0, Zone::Battlefield, data.power, data.toughness);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Avacynian Priest".into();
        // summoning_sick defaults to true on creation — do NOT clear it
        id
    };

    // Verify it has summoning sickness
    assert!(state.get_object(priest).unwrap().summoning_sick,
        "Priest should have summoning sickness");

    // Add mana for the {1} activation cost
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Place a target creature for the opponent
    let _target = ready_creature(&mut state, P1, 3, 3);

    // Get legal actions — the Priest's tap ability should NOT be available
    let legal = engine::legal_actions(&state, &registry);
    let has_priest_ability = legal.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == priest)
    });

    // BUG: This assertion should pass (ability should NOT be available)
    // but currently fails because engine doesn't check summoning sickness for tap abilities
    assert!(!has_priest_ability,
        "Priest with summoning sickness should NOT be able to activate {{T}} ability");
}

// ═══════════════════════════════════════════════════════════════
// SUBTYPE CHECK MISSES TOKENS
// Cards that check subtypes via registry.card_data() miss tokens,
// which store subtypes on obj.subtypes instead.
// ═══════════════════════════════════════════════════════════════

/// Bug: Victim of Night can target Vampire tokens.
/// Oracle: "Destroy target non-Vampire, non-Werewolf, non-Zombie creature."
/// The is_valid_target check uses registry.card_data() which returns None for
/// tokens, so the subtype exclusion fails and tokens are targetable.
#[test]
fn bug_victim_of_night_can_target_vampire_token() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a Vampire token (like Bloodline Keeper creates)
    let vampire_token = state.create_token_with_subtypes(
        "Vampire", P1, 2, 2,
        vec![Color::Black],
        vec![CardType::Creature],
        vec![],
        vec!["Vampire".into()],
    );
    if let Some(obj) = state.get_object_mut(vampire_token) {
        obj.summoning_sick = false;
    }

    // Verify token has Vampire subtype
    assert!(state.get_object(vampire_token).unwrap().subtypes.contains(&"Vampire".into()),
        "Token should have Vampire subtype");

    // Cast Victim of Night targeting the Vampire token
    let victim = castable_spell(&mut state, &registry, "Victim of Night", P0);

    // Check if the Vampire token is a valid target
    let behavior = registry.get(
        registry.get_id_by_name("Victim of Night").unwrap()
    ).unwrap();
    let is_valid = behavior.is_valid_target(
        &state, P0, &Target::Object(vampire_token), &registry
    );

    // BUG: Token should NOT be a valid target (it's a Vampire),
    // but is_valid_target only checks registry which has no data for tokens
    assert!(!is_valid,
        "Vampire token should NOT be a valid target for Victim of Night");
}

// ═══════════════════════════════════════════════════════════════
// ENGINE: TRIGGER ZONE CHECK
// ETB triggers should still resolve if the source leaves before resolution.
// Per MTG rules, an ETB trigger goes on the stack and resolves independently.
// ═══════════════════════════════════════════════════════════════

/// Bug: ETB triggers are suppressed when source leaves battlefield before resolution.
/// The trigger resolution in triggers.rs:893-899 checks zone == Battlefield.
/// Per MTG rules, ETB triggers resolve independently — removing the source
/// doesn't prevent the trigger from resolving.
///
/// This test goes through the trigger dispatch system (not calling handler directly)
/// to demonstrate the bug is in the trigger resolution path.
#[test]
fn bug_etb_trigger_suppressed_when_source_leaves() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 some library cards to mill
    for _ in 0..10 {
        let card = state.create_object(
            registry.get_id_by_name("Grizzly Bears").unwrap(),
            P0, Zone::Library, Some(2), Some(2),
        );
        state.get_player_mut(P0).library_order.push(card);
    }
    let lib_before = state.get_player(P0).library_order.len();

    // Cast Armored Skaab — this will put it on the stack
    let skaab = castable_spell(&mut state, &registry, "Armored Skaab", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: skaab, targets: vec![], sacrifice: None, exile_count: None, alternative_cost: None },
        &registry,
    );
    // Resolve — moves to battlefield, queues ETB trigger
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Skaab is now on battlefield with ETB trigger pending
    assert_eq!(state.get_object(skaab).unwrap().zone, Zone::Battlefield);

    // Kill Skaab before the ETB trigger resolves (move to graveyard)
    state.move_object(skaab, Zone::Graveyard);
    assert_eq!(state.get_object(skaab).unwrap().zone, Zone::Graveyard);

    // Process pending triggers — the ETB mill should still happen
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    let lib_after = state.get_player(P0).library_order.len();

    // BUG: Mill doesn't happen because trigger resolution checks zone == Battlefield
    assert_eq!(lib_before - lib_after, 4,
        "ETB trigger should still mill 4 even after Skaab left the battlefield");
}

// ═══════════════════════════════════════════════════════════════
// AUTO-SELECTS INSTEAD OF PLAYER CHOICE
// "Target player" means the player chooses; auto-selecting opponent is wrong.
// ═══════════════════════════════════════════════════════════════

/// Bug: Falkenrath Noble auto-targets the opponent for life drain.
/// Oracle: "target player loses 1 life and you gain 1 life"
/// "Target player" means the controller chooses which player to target,
/// including potentially themselves. The code does state.opponent(controller)
/// without presenting a choice.
#[test]
fn bug_falkenrath_noble_auto_targets_opponent() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Falkenrath Noble for P0
    let noble = named_creature(&mut state, &registry, "Falkenrath Noble", P0);

    // Place a creature for P1 and kill it to trigger Noble
    let victim = ready_creature(&mut state, P1, 1, 1);
    mtg_engine::destruction::sacrifice(&mut state, victim, &registry);
    mtg_engine::sba::check_state_based_actions(&mut state);

    // Process the death trigger
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // The Noble's trigger should present a choice of which player to target.
    // If it auto-targeted the opponent, P1's life will already be 19 and
    // there won't be an AwaitingAction for player choice.
    //
    // BUG: Noble auto-selects opponent — no choice is presented
    let p1_life = state.get_player(P1).life;
    let awaiting = state.awaiting_action.is_some();

    // Either there should be an awaiting action (choice pending)
    // OR if it already resolved, it should have targeted correctly.
    // The bug is that it resolves WITHOUT presenting a choice.
    assert!(awaiting || p1_life == 20,
        "Noble should either present target choice (awaiting_action) or not have auto-drained yet. P1 life: {}, awaiting: {}",
        p1_life, awaiting);
}

// ═══════════════════════════════════════════════════════════════
// ENGINE: SIMULTANEOUS DEATH
// When multiple creatures die at once, death triggers should fire for each.
// ═══════════════════════════════════════════════════════════════

/// Bug: Falkenrath Noble only triggers once when dying simultaneously with others.
/// Oracle + ruling: "If Falkenrath Noble and another creature die at the same time,
/// Falkenrath Noble's triggered ability will trigger for each of them."
/// The engine processes deaths sequentially; by the time other creatures' deaths are
/// processed, Noble is already in the graveyard and the zone check fails.
#[test]
fn bug_simultaneous_death_triggers_only_fire_once() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let p0_life_before = state.get_player(P0).life;

    // Place Falkenrath Noble and two other creatures for P0
    let noble = named_creature(&mut state, &registry, "Falkenrath Noble", P0);
    let creature1 = ready_creature(&mut state, P0, 1, 1);
    let creature2 = ready_creature(&mut state, P0, 1, 1);

    // Kill all three simultaneously (board wipe — mark lethal damage)
    for id in [noble, creature1, creature2] {
        if let Some(obj) = state.get_object_mut(id) {
            obj.damage_marked = 99;
        }
    }

    // Run SBAs — all three die at once
    mtg_engine::sba::check_state_based_actions(&mut state);

    // Process death triggers
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // Noble should have triggered 3 times (once for itself via SelfDies,
    // once for each of the two other creatures via AnyCreatureDies).
    // Each trigger drains 1 life from opponent and gains 1 for controller.
    // Expected: P0 gains 3 life (20 -> 23), P1 loses 3 life (20 -> 17)
    let p0_life = state.get_player(P0).life;
    let p1_life = state.get_player(P1).life;

    // BUG: Only triggers once (for SelfDies), so P0 gains 1 and P1 loses 1
    assert_eq!(p0_life, p0_life_before + 3,
        "Noble should trigger 3 times (self + 2 others). P0 life: {} (expected {})",
        p0_life, p0_life_before + 3);
}

// ═══════════════════════════════════════════════════════════════
// MISSING SHUFFLE
// Cards that search libraries must shuffle afterward.
// ═══════════════════════════════════════════════════════════════

/// Bug: Ghost Quarter doesn't shuffle the library after the land search.
/// Oracle: "put it onto the battlefield, then shuffle."
/// The code finds and places the land but never calls library_order.shuffle().
#[test]
fn bug_ghost_quarter_missing_shuffle() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Ghost Quarter for P0
    let gq = named_creature(&mut state, &registry, "Ghost Quarter", P0);

    // Place a target land for P1
    let target_land = {
        let card_id = registry.get_id_by_name("Forest").unwrap();
        let id = state.create_object(card_id, P1, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Forest".into();
        id
    };

    // Put specific basic lands in P1's library in a known order
    let plains_ids: Vec<_> = (0..5).map(|_| {
        let card_id = registry.get_id_by_name("Plains").unwrap();
        let id = state.create_object(card_id, P1, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = "Plains".into();
        state.get_player_mut(P1).library_order.push(id);
        id
    }).collect();

    // Record the library order
    let lib_order_before: Vec<_> = state.get_player(P1).library_order.clone();

    // Activate Ghost Quarter targeting the land
    // (This is complex to set up through the engine, so let's call the behavior directly)
    let behavior = registry.get(state.get_object(gq).unwrap().card_id).unwrap();
    // Sacrifice Ghost Quarter
    state.move_object(gq, Zone::Graveyard);
    // Destroy the target land and do the search
    behavior.on_activate_ability(&mut state, gq, 1, &[Target::Object(target_land)], &registry);

    // Check if library was shuffled (order should be different)
    let lib_order_after: Vec<_> = state.get_player(P1).library_order.clone();

    // With 5 cards, the probability of the same order after shuffle is 1/120.
    // If order is identical, shuffle didn't happen.
    // BUG: Library is not shuffled after search
    assert_ne!(lib_order_before, lib_order_after,
        "Library should be shuffled after Ghost Quarter search (order unchanged)");
}

// ═══════════════════════════════════════════════════════════════
// ENGINE: FORCE-ATTACK MISSING "IF ABLE" CHECKS
// Creatures forced to attack should respect Pacifism/can't-attack effects.
// ═══════════════════════════════════════════════════════════════

/// Bug: Bloodcrazed Neonate with Pacifism is still forced to attack.
/// Oracle: "This creature attacks each combat if able."
/// "If able" means the creature must actually be able to attack.
/// Pacifism prevents attacking, so the force-attack should be skipped.
#[test]
fn bug_force_attack_ignores_cant_attack() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    // Place Bloodcrazed Neonate (has ForceAttack)
    let neonate = named_creature(&mut state, &registry, "Bloodcrazed Neonate", P0);

    // Give it Pacifism (PreventAttack)
    if let Some(obj) = state.get_object_mut(neonate) {
        obj.instance_continuous_effects = Some(vec![
            ContinuousEffect::PreventAttack { scope: EffectScope::OnSelf },
        ]);
    }

    // Get legal actions — neonate should NOT be forced to attack
    let legal = engine::legal_actions(&state, &registry);

    // Check if the combat prompt forces the neonate to attack
    if let Some(ref prompt) = legal.combat_prompt {
        match prompt {
            mtg_engine::actions::CombatPrompt::ChooseAttackers { must_attack, .. } => {
                // BUG: Engine forces attack even with PreventAttack
                assert!(!must_attack.contains(&neonate),
                    "Neonate with Pacifism should NOT be forced to attack");
            }
            _ => {}
        }
    }
}

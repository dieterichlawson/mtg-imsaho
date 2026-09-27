//! Tests for summoning sickness rules (rule 302.6).

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

/// Rule 302.6: Summoning sickness clears at the beginning of your untap step.
#[test]
fn summoning_sickness_clears_at_own_untap() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    let creature = sick_creature(&mut state, P0, 3, 3);
    assert!(state.get_object(creature).unwrap().summoning_sick);

    // Advance to P1's untap — P0's creature should still be sick.
    engine::advance_step(&mut state, &registry);
    assert_eq!(state.active_player, P1);
    assert!(state.get_object(creature).unwrap().summoning_sick,
        "Creature should still be sick during opponent's untap");

    // Advance through P1's entire turn to get back to P0's untap.
    loop {
        engine::advance_step(&mut state, &registry);
        if state.step == Step::Untap && state.active_player == P0 {
            break;
        }
    }

    assert!(!state.get_object(creature).unwrap().summoning_sick,
        "Creature should no longer be sick after controller's untap step");
}

/// Summoning sickness is set when a creature enters the battlefield
/// from any zone.
#[test]
fn entering_battlefield_gives_summoning_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Battlefield, &registry);
    assert!(state.get_object(creature).unwrap().summoning_sick);
}

/// Leaving and re-entering the battlefield resets summoning sickness.
#[test]
fn re_entering_battlefield_resets_summoning_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Hand, &registry);
    state.move_object(creature, Zone::Battlefield, &registry);
    assert!(state.get_object(creature).unwrap().summoning_sick,
        "Should be sick again after re-entering battlefield");
}

/// Summoning sickness prevents attacking but NOT blocking.
#[test]
fn sick_creature_cant_attack_but_can_block() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);

    assert!(!combat::eligible_attackers(&state, P0, &reg).contains(&creature),
        "Sick creature should not be able to attack");
    assert!(combat::eligible_blockers(&state, P0, &reg).contains(&creature),
        "Sick creature should be able to block");
}

/// Summoning sickness is cleared when leaving the battlefield.
#[test]
fn leaving_battlefield_clears_sickness() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = sick_creature(&mut state, P0, 2, 2);
    assert!(state.get_object(creature).unwrap().summoning_sick);

    state.move_object(creature, Zone::Graveyard, &registry);
    assert!(!state.get_object(creature).unwrap().summoning_sick,
        "Summoning sickness should be cleared when leaving battlefield");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

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

    assert!(!has_priest_ability,
        "Priest with summoning sickness should NOT be able to activate {{T}} ability");
}

/// CR 613.10c-adjacent bookkeeping: "gaining" control of a permanent you
/// already control is not a control change — it must not re-apply the
/// summoning-sickness reset that a real change of controller causes.
#[test]
fn a_control_change_to_the_same_controller_is_a_no_op() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let veteran = ready_creature(&mut state, P0, 2, 2);
    assert!(!state.get_object(veteran).unwrap().summoning_sick);

    state.change_control(veteran, P0);

    assert!(!state.get_object(veteran).unwrap().summoning_sick,
        "no controller changed, so no summoning sickness");
    let _ = reg;
}

/// CR 302.6 has one answer, and every seat is handed that answer.
///
/// `Object::summoning_sick` is the raw fact — came under this controller's
/// control this turn — and it is set on every permanent that entered, so it
/// is not the question any surface wants. Three surfaces read it as "can't
/// attack" anyway: the terminal said `[S]` on a just-resolved planeswalker
/// (#221) and on an attacking hasty creature (#139), the browser page
/// painted the sickness badge on an attacking Manor Skeleton while its
/// inspector listed `Haste` beside `Summoning sick` (#604), and the LLM
/// seat was handed `haste [S]` against its own legend's "`S` = can't
/// attack" (#605). The terminal's fix was a helper inside `cli.rs`, which
/// is exactly why the other two never got it.
///
/// So the engine answers it — `has_summoning_sickness` — the view carries
/// the answer, and this is the test that the answer is the same one the
/// gates enforce. A surface that agrees with `eligible_attackers` and
/// `can_pay_tap_cost` cannot contradict the game it is showing.
#[test]
fn the_view_says_summoning_sick_about_exactly_the_permanents_the_rules_restrict() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Haste: the flag is set for its whole first turn and the rules do not
    // restrict it. Manor Skeleton is {1}{B} 1/1 haste.
    let hasty = named_permanent(&mut state, &registry, "Manor Skeleton", P0);
    state.get_object_mut(hasty).unwrap().summoning_sick = true;
    // No haste: restricted.
    let fresh = named_permanent(&mut state, &registry, "Walking Corpse", P0);
    state.get_object_mut(fresh).unwrap().summoning_sick = true;
    // Not a creature: the flag is set and means nothing (#221).
    let walker = named_permanent(&mut state, &registry, "Liliana of the Veil", P0);
    state.get_object_mut(walker).unwrap().summoning_sick = true;
    set_loyalty(&mut state, walker, 3);
    // A creature that has been here since the turn began.
    let settled = named_permanent(&mut state, &registry, "Walking Corpse", P0);
    state.get_object_mut(settled).unwrap().summoning_sick = false;

    for (id, restricted, what) in [
        (hasty, false, "a creature with haste (CR 702.10b)"),
        (fresh, true, "a creature that entered this turn (CR 302.6)"),
        (walker, false, "a planeswalker that just resolved (#221)"),
        (settled, false, "a creature that was already here"),
    ] {
        assert_eq!(state.has_summoning_sickness(id, &registry), restricted,
            "the engine about {what}");
    }

    let view = mtg_engine::view::GameView::for_player(&state, P0, &registry);
    for perm in &view.battlefield {
        assert_eq!(
            perm.affected_by_summoning_sickness,
            state.has_summoning_sickness(perm.object_id, &registry),
            "the view disagrees with the engine about {} (#{})",
            perm.name, perm.object_id.0);
    }
    let flagged: Vec<&str> = view.battlefield.iter()
        .filter(|p| p.affected_by_summoning_sickness)
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(flagged, vec!["Walking Corpse"],
        "only the creature the rules restrict is flagged: {flagged:?}");

    // And the flag agrees with the two gates that enforce the rule, which is
    // what makes it safe for a surface to print "can't attack" from it.
    let mut combat_state = game_at_step(Step::DeclareAttackers, P0);
    let hasty = named_permanent(&mut combat_state, &registry, "Manor Skeleton", P0);
    let fresh = named_permanent(&mut combat_state, &registry, "Walking Corpse", P0);
    for id in [hasty, fresh] {
        combat_state.get_object_mut(id).unwrap().summoning_sick = true;
    }
    let eligible = combat::eligible_attackers(&combat_state, P0, &registry);
    let combat_view = mtg_engine::view::GameView::for_player(&combat_state, P0, &registry);
    for perm in &combat_view.battlefield {
        if !combat_state.is_creature(perm.object_id, &registry) {
            continue;
        }
        assert_eq!(
            !perm.affected_by_summoning_sickness,
            eligible.contains(&perm.object_id),
            "{} wears the sickness mark but {} declared as an attacker",
            perm.name,
            if eligible.contains(&perm.object_id) { "may be" } else { "may not be" });
    }
    assert!(eligible.contains(&hasty), "the hasty creature attacks the turn it enters");
    assert!(!eligible.contains(&fresh), "the other one does not");
}

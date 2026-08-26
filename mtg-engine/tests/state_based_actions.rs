//! Tests for state-based actions (rule 704).

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::GameResult;
use mtg_engine::types::*;

/// Rule 104.4a: If both players reach 0 life simultaneously, it's a draw.
#[test]
fn simultaneous_life_loss_is_draw() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = 0;
    state.players[1].life = -2;

    check_state_based_actions(&mut state, &reg);

    assert!(state.players[0].lost);
    assert!(state.players[1].lost);
    assert_eq!(state.result, Some(GameResult::Draw),
        "Both players at <=0 life simultaneously should be a draw (rule 104.4a)");
}

/// Rule 704.5g: Creature with damage >= toughness dies, but only when SBAs
/// are checked — not at the instant damage is dealt.
#[test]
fn creature_survives_until_sba_check() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let creature = ready_creature(&mut state, P0, 2, 3);
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    // Before SBA check, creature is still on the battlefield.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);

    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Rule 704.5b: Drawing from an empty library sets a flag. Player loses
/// when SBAs are next checked, not immediately.
#[test]
fn empty_library_loss_is_deferred_to_sba() {
    let reg = registry();
    let mut state = game_at_step(Step::Draw, P0);

    let _ = engine::draw_cards(&mut state, P0, 1, &reg);

    assert!(state.get_player(P0).has_drawn_from_empty);
    assert!(!state.get_player(P0).lost,
        "Player should not lose immediately on empty draw (rule 704.5b)");

    check_state_based_actions(&mut state, &reg);
    assert!(state.get_player(P0).lost);
}

/// Rules 704.5f and 704.5g: a creature dies when its toughness is 0 or less,
/// or when marked damage is at least its toughness. Six one-assert tests used
/// to walk this boundary a case at a time; the boundary is the point, so walk
/// it in one place where a missing case is visible.
#[test]
fn a_creature_dies_exactly_at_the_toughness_boundary() {
    // (toughness, damage marked, dies?)
    const CASES: &[(i32, u32, bool)] = &[
        (0, 0, true),    // 704.5f: zero toughness, no damage needed
        (-1, 0, true),   // 704.5f: negative toughness too
        (4, 3, false),   // 704.5g: below lethal
        (3, 3, true),    // exactly lethal
        (1, 100, true),  // overkill is still just lethal
        (3, 0, false),   // undamaged
    ];
    let reg = registry();
    for &(toughness, damage, dies) in CASES {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let creature = state.create_object(CardId(99), P0, Zone::Battlefield, Some(2), Some(toughness));
        state.get_object_mut(creature).unwrap().damage_marked = damage;

        check_state_based_actions(&mut state, &reg);

        let expected = if dies { Zone::Graveyard } else { Zone::Battlefield };
        assert_eq!(state.get_object(creature).unwrap().zone, expected,
            "a {toughness}-toughness creature with {damage} damage should {}",
            if dies { "die" } else { "survive" });
    }
}

/// Rule 704.3: SBAs repeat until none are performed. Multiple SBAs
/// can happen in one check cycle.
#[test]
fn sbas_repeat_until_stable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.players[0].life = 0;
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    check_state_based_actions(&mut state, &reg);

    assert!(state.players[0].lost);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Rule 704.5a: a player at 0 or less life loses. The interesting part is
/// where the line is, so state it once.
#[test]
fn a_player_loses_exactly_at_zero_life() {
    const CASES: &[(i32, bool)] = &[(1, false), (0, true), (-10, true)];
    let reg = registry();
    for &(life, loses) in CASES {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.players[0].life = life;

        check_state_based_actions(&mut state, &reg);

        assert_eq!(state.players[0].lost, loses,
            "a player at {life} life should {}", if loses { "lose" } else { "not lose" });
    }
}

/// No SBAs when everything is fine — check returns false.
#[test]
fn no_sbas_when_stable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P0, 3, 3);

    assert!(!check_state_based_actions(&mut state, &reg));
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Grimoire of the Dead returns ALL creature cards from all
/// graveyards, but doesn't apply the legend rule to legendary creatures
/// that are already on the battlefield.
#[test]
fn bug_grimoire_legend_rule_not_applied() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place a legendary creature on P0's battlefield
    let existing = named_creature(&mut state, &registry, "Grimgrin, Corpse-Born", P0);

    // Put another copy of the same legendary in P1's graveyard
    let _graveyard_copy = {
        let card_id = registry.get_id_by_name("Grimgrin, Corpse-Born").unwrap();
        let id = state.create_object(card_id, P1, Zone::Graveyard, Some(5), Some(5));
        state.get_object_mut(id).unwrap().name = "Grimgrin, Corpse-Born".into();
        id
    };

    // Simulate Grimoire's ability 1 (return all creatures as Zombies)
    let grimoire = named_creature(&mut state, &registry, "Grimoire of the Dead", P0);
    let behavior = registry.get(state.get_object(grimoire).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, grimoire, 1, &[], &registry);

    // After returning, we should have two legendary Grimgrins controlled by P0.
    let grimgrins: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name.contains("Grimgrin"))
        .map(|o| (o.id, o.is_legendary))
        .collect();
    assert_eq!(grimgrins.len(), 2,
        "Test setup: should have 2 Grimgrins on battlefield before SBA. Got: {grimgrins:?}");
    assert!(grimgrins.iter().all(|(_, leg)| *leg),
        "Both Grimgrins must have is_legendary=true for SBA to detect them. Got: {grimgrins:?}");

    // SBA should present a legend-rule choice.
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);
    assert!(state.awaiting_action.is_some(),
        "Legend rule SBA should present a choice for which Grimgrin to keep");

    // Resolve the choice: keep the existing one.
    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(existing))),
        },
        &registry,
    );

    // Count Grimgrins on battlefield
    let grimgrin_count = new_state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name.contains("Grimgrin"))
        .count();

    assert_eq!(grimgrin_count, 1,
        "Legend rule should leave only 1 Grimgrin. Found: {grimgrin_count}");
}

/// Bug: When a board wipe destroys both a Human and Angelic Overseer
/// simultaneously, the SBA processes them sequentially. The Human
/// might die first, causing Overseer to lose indestructible before
/// its own destruction is checked.
#[test]
fn bug_angelic_overseer_sba_ordering() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Angelic Overseer (indestructible while you control a Human)
    let overseer = named_creature(&mut state, &registry, "Angelic Overseer", P0);

    // Place a Human
    let human = named_creature(&mut state, &registry, "Champion of the Parish", P0);

    // Deal lethal damage to both simultaneously (board wipe)
    if let Some(obj) = state.get_object_mut(overseer) {
        obj.damage_marked = 99;
    }
    if let Some(obj) = state.get_object_mut(human) {
        obj.damage_marked = 99;
    }

    // Clear events so we can track death order.
    state.events.clear();

    // Run SBAs with registry.
    // Per MTG rules 704.3: SBAs are checked simultaneously.
    // Pass 1: Human has lethal damage → dies. Overseer has lethal damage but is
    //         indestructible (Human still alive at snapshot) → survives.
    // Pass 2: Overseer still has lethal damage, no longer indestructible → dies.
    // End result: both die, but in SEPARATE SBA passes (not simultaneously).
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    let overseer_zone = state.get_object(overseer).unwrap().zone;
    let human_zone = state.get_object(human).unwrap().zone;

    // Both should be dead.
    assert_eq!(human_zone, Zone::Graveyard, "Human should die from lethal damage");
    assert_eq!(overseer_zone, Zone::Graveyard,
        "Overseer dies on second SBA pass (no longer indestructible after Human dies)");

    // Verify they died in SEPARATE SBA passes (not simultaneously).
    // With the snapshot fix, the Human's CreatureDied event comes first,
    // then triggers could process, then the Overseer dies on the next pass.
    // We verify by checking that both CreatureDied events exist.
    let death_events: Vec<_> = state.events.iter()
        .filter_map(|e| {
            if let mtg_engine::events::GameEvent::CreatureDied { object, .. } = e {
                Some(*object)
            } else {
                None
            }
        })
        .collect();
    assert!(death_events.contains(&human), "Human should have a CreatureDied event");
    assert!(death_events.contains(&overseer), "Overseer should have a CreatureDied event");
}

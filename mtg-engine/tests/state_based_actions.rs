//! State-based actions (CR 704). Includes the legend rule (CR 704.5j) and
//! counter annihilation (CR 704.5q), which are checked here and nowhere else.

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::GameResult;
use mtg_engine::types::*;
use mtg_engine::state::PendingEffect;

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
    let existing = named_permanent(&mut state, &registry, "Grimgrin, Corpse-Born", P0);

    // Put another copy of the same legendary in P1's graveyard
    let _graveyard_copy = {
        let card_id = registry.get_id_by_name("Grimgrin, Corpse-Born").unwrap();
        let id = state.create_object(card_id, P1, Zone::Graveyard, Some(5), Some(5));
        state.get_object_mut(id).unwrap().name = "Grimgrin, Corpse-Born".into();
        id
    };

    // Simulate Grimoire's ability 1 (return all creatures as Zombies)
    let grimoire = named_permanent(&mut state, &registry, "Grimoire of the Dead", P0);
    activate_via_hooks(&mut state, &registry, grimoire, 1, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // After returning, we should have two legendary Grimgrins controlled by P0.
    let grimgrins: Vec<_> = state.objects_in_id_order().into_iter()
        .filter(|o| o.zone == Zone::Battlefield && o.name.contains("Grimgrin"))
        .map(|o| o.id)
        .collect();
    assert_eq!(grimgrins.len(), 2,
        "Test setup: should have 2 Grimgrins on battlefield before SBA. Got: {grimgrins:?}");
    // Asks the property, not the `is_legendary` cache this used to assert.
    // Only the ordinary "resolve a permanent spell" path ever set that flag, so
    // requiring it here was requiring every card that reanimates a legend to
    // remember to stamp it — which is the bug the legend rule keeps hitting.
    assert!(grimgrins.iter().all(|&id| state.is_legendary(id, &registry)),
        "both Grimgrins are legendary, however they got to the battlefield");

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

/// The legend rule raises a keep-choice and returns — and the engine's SBA
/// loop calls `check_state_based_actions` again before anyone can answer.
/// The re-check used to find the same duplicates, re-present the same
/// choice, and report an action taken, every time: an infinite loop with no
/// decision points (found by seeded fuzzing — rb-vampires vs ub-zombies,
/// seed 74, two Grimgrins). While a choice is pending, an SBA re-check must
/// be a no-op (CR 704.3: SBAs are checked when a player is about to receive
/// priority, and a player waiting on a choice is not).
#[test]
fn a_pending_legend_choice_stops_the_sba_loop() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let keeper = named_permanent(&mut state, &registry, "Grimgrin, Corpse-Born", P0);
    let _double = named_permanent(&mut state, &registry, "Grimgrin, Corpse-Born", P0);

    assert!(check_state_based_actions(&mut state, &registry),
        "the first check applies the legend rule");
    assert!(state.awaiting_action.is_some(), "and raises the keep-choice");

    // The engine's `while check_state_based_actions` loop calls again while
    // the choice is still pending. This must not count as an action, or the
    // loop never reaches the player.
    assert!(!check_state_based_actions(&mut state, &registry),
        "a re-check while the choice is pending is a no-op");
    assert!(state.awaiting_action.is_some(), "and leaves the pending choice standing");

    // Answering the choice settles the board to one Grimgrin.
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(keeper))),
        },
        &registry,
    );
    let grimgrins = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name.contains("Grimgrin"))
        .count();
    assert_eq!(grimgrins, 1, "the kept Grimgrin survives alone");
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
    let overseer = named_permanent(&mut state, &registry, "Angelic Overseer", P0);

    // Place a Human
    let human = named_permanent(&mut state, &registry, "Champion of the Parish", P0);

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

/// CR 702.12b: indestructible means "destroy" effects and lethal damage do not
/// destroy this permanent. It is not general immunity — 0 toughness (CR 704.5f)
/// still kills, and sacrificing is not destruction at all (CR 701.17b).
///
/// Seven tests used to cover these four claims, split across two files, and the
/// only thing that distinguished the two halves was whether indestructible was
/// printed on the object or granted by a continuous effect. That distinction is
/// worth exactly one axis of a table, not a second copy of every case.
#[test]
fn indestructible_stops_destruction_and_lethal_damage_but_nothing_else() {
    #[derive(Clone, Copy)]
    enum Grant { Printed, Granted }

    let reg = registry();
    for grant in [Grant::Printed, Grant::Granted] {
        // Build a 4/4 that is indestructible one way or the other.
        let setup = |toughness: i32| {
            let mut state = game_at_step(Step::PrecombatMain, P0);
            // CardId(9999) has no registry entry, so `obj.keywords` is where a
            // printed keyword lives for it (a card with a registry face reads
            // its keywords from that face instead).
            let creature = ready_creature(&mut state, P0, 4, toughness);
            match grant {
                Grant::Printed => {
                    state.get_object_mut(creature).unwrap().keywords = vec![Keyword::Indestructible];
                }
                Grant::Granted => state.until_end_of_turn.push(
                    mtg_engine::state::TemporaryEffect::GrantKeyword {
                        target: creature,
                        keyword: Keyword::Indestructible,
                    },
                ),
            }
            assert!(state.has_keyword(creature, Keyword::Indestructible, &reg),
                "test precondition: the creature is indestructible");
            (state, creature)
        };

        // "Destroy" does nothing.
        let (mut state, creature) = setup(4);
        assert_eq!(mtg_engine::destruction::try_destroy(&mut state, creature, &reg),
            mtg_engine::destruction::DestroyResult::Indestructible);
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
            "try_destroy must not move an indestructible permanent");

        // Lethal damage does not destroy it, and the damage stays marked
        // (CR 704.5g checks it every time SBAs run).
        let (mut state, creature) = setup(4);
        state.get_object_mut(creature).unwrap().damage_marked = 10;
        check_state_based_actions(&mut state, &reg);
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
            "lethal damage does not destroy an indestructible creature");
        assert_eq!(state.get_object(creature).unwrap().damage_marked, 10,
            "the damage stays marked rather than being cleared");

        // Deathtouch damage is still just damage (CR 704.5h destroys, and
        // destruction is what indestructible ignores).
        let (mut state, creature) = setup(4);
        {
            let obj = state.get_object_mut(creature).unwrap();
            obj.damage_marked = 1;
            obj.dealt_deathtouch_damage = true;
        }
        check_state_based_actions(&mut state, &reg);
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
            "deathtouch destroys, so indestructible survives it");

        // 0 toughness is not destruction — it dies (CR 704.5f).
        let (mut state, creature) = setup(0);
        check_state_based_actions(&mut state, &reg);
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
            "indestructible does not save a creature from 0 toughness");

        // Neither is sacrificing (CR 701.17b).
        let (mut state, creature) = setup(4);
        assert!(mtg_engine::destruction::sacrifice(&mut state, creature, &reg),
            "sacrifice succeeds on an indestructible creature");
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
    }
}

// -------------------------------------------------------------------------
// The legend rule (CR 704.5j)
// -------------------------------------------------------------------------

/// Two legendary creatures with the same name — one should be removed by SBAs.
/// Uses the registry to check legendary status via CardData.supertypes.
/// Since no legendary cards exist in the registry yet, we simulate by
/// setting the `is_legendary` flag directly on the `GameObject`.
#[test]
fn legend_rule_removes_duplicate() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create two legendary creatures with the same name.
    let card_id = CardId(200);
    let legend1 = state.create_object(card_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend1).unwrap().name = "Thalia".into();
    state.get_object_mut(legend1).unwrap().summoning_sick = false;
    state.get_object_mut(legend1).unwrap().is_legendary = true;

    let legend2 = state.create_object(card_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend2).unwrap().name = "Thalia".into();
    state.get_object_mut(legend2).unwrap().summoning_sick = false;
    state.get_object_mut(legend2).unwrap().is_legendary = true;

    check_state_based_actions(&mut state, &reg);

    // SBA should have set up a legend-rule choice.
    assert!(state.awaiting_action.is_some(),
        "Legend rule SBA should present a choice for which legendary to keep");

    // Resolve the choice: keep legend1.
    let new_state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(legend1))),
        },
        &reg,
    );

    // Track the two by id. Identifying them by the hand-set name does not
    // work: CR 400.7 restores an object's PRINTED name when it leaves the
    // battlefield, so the loser reverts to whatever `card_id` actually names.
    assert_eq!(new_state.get_object(legend1).unwrap().zone, Zone::Battlefield,
        "Legend rule: the kept legendary stays (CR 704.5k)");
    assert_eq!(new_state.get_object(legend2).unwrap().zone, Zone::Graveyard,
        "Legend rule: the duplicate goes to the graveyard");
}

/// Legendary permanents with DIFFERENT names are fine — both stay.
#[test]
fn legend_rule_different_names_coexist() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let legend1 = state.create_object(CardId(200), P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend1).unwrap().name = "Thalia".into();
    state.get_object_mut(legend1).unwrap().is_legendary = true;

    let legend2 = state.create_object(CardId(201), P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(legend2).unwrap().name = "Geist".into();
    state.get_object_mut(legend2).unwrap().is_legendary = true;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(legend1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(legend2).unwrap().zone, Zone::Battlefield);
}

/// Different players can each control a legendary with the same name.
#[test]
fn legend_rule_different_controllers_ok() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = CardId(200);
    let legend_p0 = state.create_object(card_id, P0, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend_p0).unwrap().name = "Thalia".into();
    state.get_object_mut(legend_p0).unwrap().is_legendary = true;

    let legend_p1 = state.create_object(card_id, P1, Zone::Battlefield, Some(3), Some(3));
    state.get_object_mut(legend_p1).unwrap().name = "Thalia".into();
    state.get_object_mut(legend_p1).unwrap().is_legendary = true;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(legend_p0).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(legend_p1).unwrap().zone, Zone::Battlefield);
}

/// Non-legendary permanents with the same name are unaffected.
#[test]
fn legend_rule_ignores_non_legendary() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(c1).unwrap().name = "Grizzly Bears".into();
    let c2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(c2).unwrap().name = "Grizzly Bears".into();

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Battlefield);
}

// -------------------------------------------------------------------------
// +1/+1 and -1/-1 counters annihilate (CR 704.5q)
// -------------------------------------------------------------------------

/// Equal numbers of +1/+1 and -1/-1 counters: all removed.
/// CR 704.5q: if a permanent has both +1/+1 and -1/-1 counters, N of each are
/// removed, where N is the smaller of the two counts. Five one-case tests used
/// to walk this; the interesting part is the arithmetic at the boundary, so it
/// reads better as the table it always was.
#[test]
fn plus_and_minus_counters_annihilate_in_pairs() {
    // (base size, start +1/+1, start -1/-1, left +1/+1, left -1/-1)
    // Each creature is big enough to survive its own case, so the counter
    // arithmetic is what is under test rather than the toughness check.
    const CASES: &[(i32, u32, u32, u32, u32)] = &[
        (2, 3, 3, 0, 0),   // equal counts cancel out entirely
        (2, 5, 2, 3, 0),   // more plus: the surplus stays
        (5, 1, 4, 0, 3),   // more minus: 5/5 down to 2/2, still alive
        (2, 3, 0, 3, 0),   // only one kind — nothing to annihilate
    ];
    let reg = registry();
    for &(base, plus, minus, left_plus, left_minus) in CASES {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let creature = ready_creature(&mut state, P0, base, base);
        state.add_counters(creature, CounterType::PlusOnePlusOne, plus);
        state.add_counters(creature, CounterType::MinusOneMinusOne, minus);

        check_state_based_actions(&mut state, &reg);

        assert_eq!(state.get_counter_count(creature, CounterType::PlusOnePlusOne), left_plus,
            "{plus} +1/+1 and {minus} -1/-1 should leave {left_plus} +1/+1");
        assert_eq!(state.get_counter_count(creature, CounterType::MinusOneMinusOne), left_minus,
            "{plus} +1/+1 and {minus} -1/-1 should leave {left_minus} -1/-1");
        assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
            "a {base}/{base} survives {left_minus} net -1/-1 counters");
    }
}

/// Annihilation happens before the toughness check, so it can still leave a
/// creature dead: a 1/1 with one +1/+1 and two -1/-1 annihilates down to a
/// single -1/-1 and is a 0/0.
#[test]
fn annihilation_can_still_leave_a_creature_dead() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.add_counters(creature, CounterType::PlusOnePlusOne, 1);
    state.add_counters(creature, CounterType::MinusOneMinusOne, 2);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "one -1/-1 survives the annihilation and makes the 1/1 a 0/0");
}

// -------------------------------------------------------------------------
// A creature entering as a copy is not a 0/0 (Evil Twin)
// -------------------------------------------------------------------------

fn reg() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Enter an Evil Twin through the real entry chokepoint (move_object).
fn enter_twin(state: &mut mtg_engine::state::GameState, r: &CardRegistry) -> mtg_engine::ids::ObjectId {
    let card = r.get_id_by_name("Evil Twin").unwrap();
    let twin = state.create_object(card, P0, Zone::Hand, Some(0), Some(0));
    state.get_object_mut(twin).unwrap().name = "Evil Twin".into();
    state.move_object(twin, Zone::Battlefield, r);
    twin
}

/// The guard is armed at entry, so SBA doesn't kill the 0/0 before the copy
/// choice resolves.
#[test]
fn guard_armed_at_entry_protects_before_copy_resolves() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let twin = enter_twin(&mut state, &r);

    assert!(state.get_object(twin).unwrap().entering_copy_source,
        "move_object must arm the copy-guard at entry");
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Battlefield,
        "the 0/0 must survive SBA while the copy choice is pending");
}

/// After copying, the guard is disarmed and the permanent is once again
/// subject to SBA death.
#[test]
fn copy_success_disarms_guard_and_is_mortal_again() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bears = named_permanent(&mut state, &r, "Grizzly Bears", P1);
    let twin = enter_twin(&mut state, &r);

    // Resolve the copy onto Grizzly Bears.
    engine::apply_pending_effect(
        &mut state, &Target::Object(bears),
        &PendingEffect::CopyCreature { source_id: twin }, &r,
    );
    assert!(!state.get_object(twin).unwrap().entering_copy_source,
        "copy resolution must disarm the guard");

    // It's now a 2/2 — lethal damage plus SBA must kill it.
    state.get_object_mut(twin).unwrap().damage_marked = 5;
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "a resolved Evil Twin must die to lethal damage like any creature");
}

/// Declining the copy disarms the guard, so the printed 0/0 dies to SBA.
#[test]
fn declining_copy_lets_the_0_0_die() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _other = named_permanent(&mut state, &r, "Grizzly Bears", P1);
    let twin = enter_twin(&mut state, &r);

    // Present the copy choice, then decline it.
    let behavior = r.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &r);
    assert!(state.awaiting_action.is_some(), "copy choice should be pending");

    let mut state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) },
        &r,
    );
    assert!(!state.get_object(twin).unwrap().entering_copy_source,
        "declining must disarm the guard");
    // The game loop runs SBAs after the choice resolves; the disarmed 0/0 dies.
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "a declined Evil Twin is a 0/0 and dies to SBA");
}

/// With no other creature to copy, the guard is disarmed and the 0/0 dies.
#[test]
fn no_target_lets_the_0_0_die() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let twin = enter_twin(&mut state, &r);

    let behavior = r.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &r);
    assert!(!state.get_object(twin).unwrap().entering_copy_source,
        "no legal target must disarm the guard");
    while mtg_engine::sba::check_state_based_actions(&mut state, &r) {}
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "an Evil Twin with nothing to copy dies to SBA");
}

/// CR 603.8: a state-triggered ability doesn't trigger again while it waits
/// on the stack — `state_trigger_on_stack` is the guard. Garruk Relentless
/// at 2 loyalty triggers once; a second SBA check while the trigger is
/// pending must not push a duplicate.
#[test]
fn a_state_trigger_on_the_stack_does_not_retrigger() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_permanent(&mut state, &registry, "Garruk Relentless", P0);
    state.get_object_mut(garruk).unwrap()
        .counters.insert(mtg_engine::types::CounterType::Loyalty, 2);

    check_state_based_actions(&mut state, &registry);
    assert_eq!(state.pending_triggers.len(), 1, "the transform trigger fires once");

    check_state_based_actions(&mut state, &registry);
    assert_eq!(state.pending_triggers.len(), 1,
        "and does not fire again while it is pending (CR 603.8)");
}

/// CR 704.3 + 704.5g: state-based actions repeat until none apply, and each
/// check reads marked damage against the creature's CURRENT toughness. So
/// when an anthem source dies of combat damage, a creature it was boosting
/// that had non-lethal damage marked shrinks and dies on the repeated check
/// — the classic Goblin King ruling. (Playtest issue #41 expected the
/// opposite; this test pins the correct behavior.)
#[test]
fn a_creature_shrunk_by_its_anthems_death_dies_of_its_marked_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Mayor of Avabruck: "Other Human creatures you control get +1/+1."
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
    assert_eq!(state.effective_toughness(pilgrim, &reg), Some(2),
        "1/1 Human boosted to 2/2 by the Mayor");

    // Each has 1 damage marked: lethal for the 1/1 Mayor, not (yet) for the
    // boosted 2/2 Pilgrim.
    state.get_object_mut(mayor).unwrap().damage_marked = 1;
    state.get_object_mut(pilgrim).unwrap().damage_marked = 1;

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(mayor).unwrap().zone, Zone::Graveyard,
        "the Mayor dies of its lethal damage on the first check");
    assert_eq!(state.get_object(pilgrim).unwrap().zone, Zone::Graveyard,
        "the repeated check finds the now-1/1 Pilgrim with 1 damage marked \
         and destroys it too (CR 704.3: checks repeat; damage stays marked)");
}

//! Turn structure: step progression, what each step does, and what the
//! cleanup step takes away.
//!
//! CR 500-514. The cleanup half is the larger one: CR 514.2 removes damage
//! and "until end of turn" effects, and CR 514.3a checks state-based actions
//! straight afterwards — so a creature kept alive only by a buff that just
//! expired dies right there, and one whose lethal damage was cleared in the
//! same breath does not.

mod common;

use common::*;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;
use mtg_engine::combat;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::TemporaryEffect;

/// Rule 502.4: No player receives priority during the untap step.
#[test]
fn no_priority_during_untap() {
    let registry = registry();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Untap);
    assert_eq!(state.priority_player, None,
        "No player should have priority during untap (rule 502.4)");
}

/// CR 106.4: a mana pool empties at the end of every step and phase, not only
/// at end of turn. Three boundaries, because "it emptied" at one of them is
/// also true of an engine that empties pools at some unrelated moment.
#[test]
fn a_mana_pool_empties_at_every_step_boundary() {
    // (step to float mana in, the step that follows it). No attackers are
    // ever declared here, so leaving DeclareAttackers skips to EndCombat
    // (CR 508.8).
    const BOUNDARIES: &[(Step, Step)] = &[
        (Step::Upkeep, Step::Draw),
        (Step::PrecombatMain, Step::BeginCombat),
        (Step::DeclareAttackers, Step::EndCombat),
    ];

    for &(from, to) in BOUNDARIES {
        let registry = registry();
        let mut state = game_at_step(from, P0);
        state.get_player_mut(P0).mana_pool.add(ManaType::Green, 3);

        engine::advance_step(&mut state, &registry);

        assert_eq!(state.step, to, "test setup: {from:?} is followed by {to:?}");
        assert_eq!(state.get_player(P0).mana_pool.total(), 0,
            "mana floated in {from:?} is gone by {to:?}");
    }
}

/// CR 103.7a: in a two-player game the player on the play skips the draw
/// STEP of their first turn — the whole step, so no priority window opens
/// in it and no card is drawn. Suppressing only the draw used to leave a
/// phantom draw step with a full priority round (issue #113).
#[test]
fn first_player_skips_the_whole_first_draw_step() {
    let registry = registry();
    let mut state = GameState::new(2);
    state.is_first_turn = true;
    state.step = Step::Upkeep;
    state.active_player = P0;
    state.priority_player = Some(P0);

    let card = state.create_object(CardId(1), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(card);

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::PrecombatMain,
        "the draw step is skipped entirely, straight to the main phase");
    assert_eq!(state.get_object(card).unwrap().zone, Zone::Library,
        "First player should skip the draw on the very first turn");
}

/// The skip is CR 103.7a's two-player rule; in another multiplayer
/// structure the first player draws normally (CR 103.7c), and the draw
/// step, once entered, always draws.
#[test]
fn multiplayer_first_player_does_not_skip_the_draw_step() {
    let registry = registry();
    let mut state = GameState::new(3);
    state.is_first_turn = true;
    state.step = Step::Upkeep;
    state.active_player = P0;
    state.priority_player = Some(P0);
    for p in 0..3 {
        state.players[p].life = 20;
    }

    let card = state.create_object(CardId(1), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(card);

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Draw);
    assert_eq!(state.get_object(card).unwrap().zone, Zone::Hand,
        "a free-for-all first player draws on turn 1 (CR 103.7c)");
}

/// Untap step: all permanents controlled by active player untap.
/// Opponent's permanents do NOT untap.
#[test]
fn untap_step_untaps_only_active_players_permanents() {
    let registry = registry();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;

    let land = state.create_object(CardId(1), P0, Zone::Battlefield, None, None);
    state.get_object_mut(land).unwrap().tapped = true;
    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().tapped = true;
    let p1_creature = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(p1_creature).unwrap().tapped = true;

    // Advance to P1's untap step.
    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Untap);
    assert_eq!(state.active_player, P1);

    assert!(!state.get_object(p1_creature).unwrap().tapped,
        "Active player's permanents should untap");
    assert!(state.get_object(land).unwrap().tapped,
        "Inactive player's permanents should NOT untap");
    assert!(state.get_object(creature).unwrap().tapped,
        "Inactive player's creatures should NOT untap");
}

/// Damage marked on creatures persists until the cleanup step.
#[test]
fn damage_persists_between_steps() {
    let registry = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let creature = ready_creature(&mut state, P0, 2, 5);
    state.get_object_mut(creature).unwrap().damage_marked = 3;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::EndCombat);
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3,
        "Damage should persist between steps");

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::PostcombatMain);
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 3);
}

/// Rule 514.2: Damage is removed during the cleanup step.
#[test]
fn damage_removed_during_cleanup() {
    let registry = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    let creature = ready_creature(&mut state, P0, 2, 5);
    state.get_object_mut(creature).unwrap().damage_marked = 4;

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Cleanup);
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be removed during cleanup (rule 514.2)");
}

/// Rule 514.1: Discard to hand size (7) during cleanup.
#[test]
fn discard_to_hand_size_during_cleanup() {
    let registry = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    for _ in 0..9 {
        state.create_object(CardId(1), P0, Zone::Hand, None, None);
    }

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Cleanup);
    assert!(matches!(
        state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::DiscardToHandSize { player, discard_count })
        if player == P0 && discard_count == 2
    ));
}

/// Submits `Action::DiscardCards` through `engine::submit_action` during
/// the cleanup discard step. Exercises the hand-size-cleanup branch of
/// the handler (is_hand_size=true), including the aggregated log message.
#[test]
fn submit_action_discard_cards_for_cleanup_moves_cards_to_graveyard() {
    use mtg_engine::actions::Action;
    use mtg_engine::state::AwaitingAction;

    let registry = registry();
    let mut state = game_at_step(Step::Cleanup, P0);
    let mut hand = Vec::new();
    for _ in 0..9 {
        hand.push(state.create_object(CardId(1), P0, Zone::Hand, None, None));
    }
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });

    let to_discard = vec![hand[0], hand[1]];
    let new_state = engine::submit_action(
        &state,
        &Action::DiscardCards { cards: to_discard.clone() },
        &registry,
    );

    assert!(new_state.awaiting_action.is_none());
    for id in &to_discard {
        assert_eq!(new_state.get_object(*id).unwrap().zone, Zone::Graveyard,
            "discarded card should be in graveyard");
    }
    let remaining_hand = new_state.objects_in_zone(Zone::Hand, P0).len();
    assert_eq!(remaining_hand, 7, "hand size should be 7 after discarding 2 of 9");
    assert!(new_state.game_log.iter().any(|e| e.message.contains("cleanup")),
        "log should annotate the discard as cleanup-driven");
}

/// Submits `Action::DiscardCards` without a `DiscardToHandSize`
/// awaiting-action (e.g. from a spell like Lay Bare or an activated
/// ability that discards). Exercises the non-cleanup branch and the
/// per-card log messages.
#[test]
fn submit_action_discard_cards_without_awaiting_logs_per_card() {
    use mtg_engine::actions::Action;

    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c1 = state.create_object(CardId(1), P0, Zone::Hand, None, None);
    let c2 = state.create_object(CardId(1), P0, Zone::Hand, None, None);

    let new_state = engine::submit_action(
        &state,
        &Action::DiscardCards { cards: vec![c1, c2] },
        &registry,
    );

    assert_eq!(new_state.get_object(c1).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(c2).unwrap().zone, Zone::Graveyard);
    let discard_msgs = new_state.game_log.iter()
        .filter(|e| e.message.contains("discarded"))
        .count();
    assert!(discard_msgs >= 2,
        "non-cleanup path should log one 'discarded' message per card (got {discard_msgs})");
}

/// No discard needed if hand size is 7 or less.
#[test]
fn no_discard_needed_at_seven_or_less() {
    let registry = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    for _ in 0..7 {
        state.create_object(CardId(1), P0, Zone::Hand, None, None);
    }

    engine::advance_step(&mut state, &registry);
    assert_eq!(state.step, Step::Cleanup);
    assert!(state.awaiting_action.is_none());
    assert_eq!(state.priority_player, None,
        "No priority during cleanup when no discard needed");
}

// -------------------------------------------------------------------------
// CR 514.3a: cleanup removes damage and until-end-of-turn effects, then SBAs run
// -------------------------------------------------------------------------

/// Both halves of the cleanup step land at once, and the state-based check
/// that follows sees the result of both.
///
/// The rows differ only in what the buff was holding up: a -1/-1 counter, which
/// is still there afterwards, or marked damage, which is not.
#[test]
fn the_cleanup_step_checks_state_based_actions_after_clearing_its_effects() {
    // (printed p/t, -1/-1 counters, damage marked, toughness the buff adds,
    //  does it survive the cleanup, why)
    const CASES: &[(i32, i32, u32, u32, i32, bool, &str)] = &[
        (1, 1, 1, 0, 1, false,
         "a -1/-1 counter outlives the buff, so 1/1 less one counter is a 0/0"),
        (2, 2, 0, 2, 1, true,
         "the damage that needed the buff is cleared by the same cleanup"),
        (3, 3, 0, 0, 2, true,
         "nothing was holding it up in the first place"),
    ];

    for &(power, toughness, counters, damage, buff, survives, why) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PostcombatMain, P0);

        let creature = ready_creature(&mut state, P0, power, toughness);
        if counters > 0 {
            state.add_counters(creature, CounterType::MinusOneMinusOne,
                u32::try_from(counters).unwrap());
        }
        state.get_object_mut(creature).unwrap().damage_marked = damage;
        state.until_end_of_turn.push(TemporaryEffect::ModifyPT {
            target: creature, power_mod: 0, toughness_mod: buff,
        });

        // Alive right now, in every row — otherwise the cleanup is not what
        // decided the outcome.
        check_state_based_actions(&mut state, &reg);
        assert_eq!(state.get_object(creature).map(|o| o.zone), Some(Zone::Battlefield),
            "{why}: it should still be alive before the cleanup");

        // Real turns mean real draw steps.
        stock_library(&mut state, &reg, P0, 20);
        stock_library(&mut state, &reg, P1, 20);
        advance_to_cleanup(&mut state, &reg);

        assert!(state.until_end_of_turn.is_empty(), "{why}: the buff is gone");
        let expected = if survives { Zone::Battlefield } else { Zone::Graveyard };
        assert_eq!(state.get_object(creature).unwrap().zone, expected, "{why}");
    }
}

// -------------------------------------------------------------------------
// What the cleanup step clears (CR 514.2), and what those effects did until then
// -------------------------------------------------------------------------

/// "Until end of turn" effects should be cleared during the cleanup step.
/// A creature with a Giant Growth (+3/+3 until EOT) should revert.
#[test]
fn cleanup_clears_until_end_of_turn_effects() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    // Simulate Giant Growth: +3/+3 until end of turn.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 3,
            toughness_mod: 3,
        },
    );

    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(5));

    advance_to_cleanup(&mut state, &reg);

    assert_eq!(
        state.effective_power(creature, &reg),
        Some(2),
        "+3/+3 should be gone after cleanup"
    );
    assert_eq!(
        state.effective_toughness(creature, &reg),
        Some(2),
        "+3/+3 should be gone after cleanup"
    );
}

/// If a creature survives only because of an "until end of turn" toughness
/// bonus, it should die in cleanup when the bonus is removed and SBAs are
/// checked. (This tests the cleanup-step SBA interaction.)
#[test]
fn creature_dies_in_cleanup_when_eot_buff_expires() {
    let reg = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    // 1/1 creature with 1 damage — alive because of +0/+1 until EOT.
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature).unwrap().damage_marked = 1;
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 0,
            toughness_mod: 1,
        },
    );

    // With the buff, effective toughness is 2, damage is 1 — survives.
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);

    advance_to_cleanup(&mut state, &reg);

    // During cleanup, damage is cleared AND the buff expires.
    // The creature is now 1/1 with 0 damage — it lives.
    // (Cleanup clears damage at the same time as removing buffs.)
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive cleanup because damage is cleared at the same time as buffs expire");
}

/// Until-end-of-turn keyword grants should expire during cleanup.
#[test]
fn cleanup_clears_keyword_grants() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword {
            target: creature,
            keyword: Keyword::Flying,
        },
    );

    assert!(state.has_keyword(creature, Keyword::Flying, &reg));

    advance_to_cleanup(&mut state, &reg);

    assert!(
        !state.has_keyword(creature, Keyword::Flying, &reg),
        "Until-end-of-turn flying should expire during cleanup"
    );
}

/// Until-end-of-turn removed keywords should be restored during cleanup.
/// Manor Gargoyle loses defender (and thus indestructible) until end of turn.
#[test]
fn cleanup_restores_removed_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 4, 4);
    // Give it defender via object keywords.
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Defender);
    assert!(state.has_keyword(creature, Keyword::Defender, &reg));

    // Remove defender until end of turn.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::RemoveKeyword {
            target: creature,
            keyword: Keyword::Defender,
        },
    );
    assert!(
        !state.has_keyword(creature, Keyword::Defender, &reg),
        "Defender should be temporarily removed"
    );

    advance_to_cleanup(&mut state, &reg);

    assert!(
        state.has_keyword(creature, Keyword::Defender, &reg),
        "Defender should be restored after cleanup"
    );
}

/// Until-end-of-turn can't-block should be cleared during cleanup.
#[test]
fn cleanup_clears_cant_block() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::CantBlock { target: creature });

    // Verify it's in the list.
    assert!(state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::CantBlock { target } if *target == creature)));

    advance_to_cleanup(&mut state, &reg);

    assert!(
        state.until_end_of_turn.is_empty(),
        "Can't-block should be cleared after cleanup"
    );
}

/// Until-end-of-turn protection should be cleared during cleanup.
#[test]
fn cleanup_clears_protection_grants() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantProtection {
            target: creature,
            filter: CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
        },
    );

    assert!(!state.until_end_of_turn.is_empty());

    advance_to_cleanup(&mut state, &reg);

    assert!(
        state.until_end_of_turn.is_empty(),
        "Protection grants should be cleared after cleanup"
    );
}

/// Until-end-of-turn control changes should be reverted during cleanup.
#[test]
fn cleanup_reverts_control_changes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 4, 4);
    assert_eq!(state.get_object(creature).unwrap().controller, P1);

    // Steal creature until end of turn.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ChangeControl { target: creature, original_controller: P1 });
    state.get_object_mut(creature).unwrap().controller = P0;
    assert_eq!(state.get_object(creature).unwrap().controller, P0);

    advance_to_cleanup(&mut state, &reg);

    assert_eq!(
        state.get_object(creature).unwrap().controller,
        P1,
        "Control should revert to original controller after cleanup"
    );
}

/// Until-end-of-turn protection should prevent a non-Human from blocking.
#[test]
fn eot_protection_prevents_blocking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    // Attacker has protection from non-Human creatures.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantProtection {
            target: attacker,
            filter: CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
        },
    );

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    // Blocker is not a Human, so can_block_attacker should reject it.
    assert!(
        !combat::can_block_attacker(&state, blocker, attacker, &reg),
        "Non-Human blocker should not be able to block creature with protection from non-Humans"
    );
}

/// Cant-block prevents a creature from appearing in eligible blockers.
#[test]
fn eot_cant_block_prevents_blocking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    // Without can't-block, blocker should be eligible.
    let eligible_before = combat::eligible_blockers(&state, P1, &reg);
    assert!(
        eligible_before.contains(&blocker),
        "Blocker should be eligible before can't-block"
    );

    // Apply can't-block.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::CantBlock { target: blocker });
    let eligible_after = combat::eligible_blockers(&state, P1, &reg);
    assert!(
        !eligible_after.contains(&blocker),
        "Blocker should not be eligible after can't-block"
    );
}

/// CR 514.3a: a cleanup step in which state-based actions were performed
/// gives players priority, and then another cleanup step happens — it does
/// not fall straight into the next turn. Before, damage marked and
/// until-end-of-turn effects created in that priority window rode into the
/// next turn, and the hand-size discard was skipped.
#[test]
fn a_cleanup_step_that_gave_priority_is_followed_by_another_cleanup_step() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // A 1/1 with a -1/-1 counter dies to the SBA check inside cleanup —
    // counters survive cleanup, damage would not.
    let doomed = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(doomed).unwrap().counters.insert(CounterType::MinusOneMinusOne, 1);

    advance_to_cleanup(&mut state, &reg);
    assert_eq!(state.get_object(doomed).unwrap().zone, Zone::Graveyard, "the SBA fired in cleanup");
    assert_eq!(state.priority_player, Some(P0), "so players get priority (CR 514.3a)");
    let turn = state.turn_number;

    // Everyone passes: the step ends — into another cleanup step.
    engine::advance_step(&mut state, &reg);
    assert_eq!((state.step, state.turn_number), (Step::Cleanup, turn),
        "another cleanup step, same turn (CR 514.3a)");
    assert_eq!(state.priority_player, None, "nothing happened this time: no priority");

    engine::advance_step(&mut state, &reg);
    assert_eq!((state.step, state.turn_number), (Step::Untap, turn + 1), "now the next turn");
}

/// CR 117.3c: the player who cast a spell or activated an ability receives
/// priority afterwards — also when the cast or activation was completed
/// through a cast-time prompt (X funding, an exile cost).
#[test]
fn a_cast_time_prompt_belongs_to_the_caster_for_priority() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    assert_eq!(engine::cast_time_prompt_player(&state), None);
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    // The non-active player is the interesting case; P1 owns the prompt here.
    state.get_object_mut(play).unwrap().owner = P1;
    state.get_object_mut(play).unwrap().controller = P1;
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        player: P1,
        source: play,
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseXFunding {
            description: "X".into(),
            options: mtg_engine::funding::FundingOptions { pool: std::collections::BTreeMap::new(), groups: vec![], max_x: 1 },
            source_id: play,
            is_ability: false,
        },
    });
    assert_eq!(engine::cast_time_prompt_player(&state), Some(P1));
}

/// The same rule, end to end through the game loop: the non-active player
/// activates Kessig Wolf Run's X ability on the opponent's turn and funds X.
/// The first priority pass after the funding must be the activator's — the
/// loop used to hand priority to the active player after any resolved
/// choice, funding included. (The activator has nothing else to do, so the
/// loop passes for them silently; the log is where the order is visible.)
#[test]
fn the_activator_keeps_priority_after_funding_an_x_ability_on_the_opponents_turn() {
    use mtg_engine::actions::{Action, ResolvedChoice};
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _target = ready_creature(&mut state, P0, 2, 2);
    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P1);
    named_permanent(&mut state, &reg, "Mountain", P1);
    named_permanent(&mut state, &reg, "Forest", P1);
    named_permanent(&mut state, &reg, "Forest", P1);
    state.priority_player = Some(P0);

    let mut calls = 0;
    engine::run_game_loop(&mut state, &reg, |state, acting, legal| {
        calls += 1;
        if let Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseXFunding { options, .. }, ..
        }) = &state.awaiting_action {
            let mut response = mtg_engine::funding::FundingResponse::default();
            for g in &options.groups {
                response.taps.insert(g.name.clone(), g.max_contribution());
            }
            return Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
        }
        if acting == P1 {
            if let Some(a) = legal.actions.iter().find(|a|
                matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == wolf_run))
            {
                return a.clone();
            }
        }
        if calls > 4 { Action::Concede } else { Action::PassPriority }
    });

    let log: Vec<&str> = state.game_log.iter().map(|l| l.message.as_str()).collect();
    let funded = log.iter().position(|m| m.starts_with("Funded X"))
        .unwrap_or_else(|| panic!("the ability was never funded:\n{}", log.join("\n")));
    let first_pass = log[funded..].iter().find(|m| m.contains("passes priority"))
        .unwrap_or_else(|| panic!("nobody passed after funding:\n{}", log.join("\n")));
    assert_eq!(*first_pass, "p1 passes priority",
        "the activator holds priority after funding (CR 117.3c):\n{}", log[funded..].join("\n"));
    assert!(log[funded..].iter().any(|m| m.contains("Kessig Wolf Run ability resolved")),
        "and the ability resolved afterwards:\n{}", log[funded..].join("\n"));
}

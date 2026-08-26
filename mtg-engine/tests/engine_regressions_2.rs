//! CR 514.3a: the cleanup step removes damage and "until end of turn" effects,
//! and then state-based actions are checked. A creature kept alive only by a
//! buff that just expired dies right there; one whose lethal damage was cleared
//! in the same breath does not.
//!
//! Also here: a spell resolves exactly once through the game loop.

mod common;

use common::*;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::TemporaryEffect;
use mtg_engine::types::*;

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

/// A spell cast through the game loop resolves once. The loop used to run
/// `resolve_top_of_stack` on a clone it then threw away before resolving for
/// real; a Lightning Bolt dealing 6 instead of 3 is what a second resolution
/// would look like.
#[test]
fn a_spell_cast_through_the_game_loop_resolves_exactly_once() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P1).life = 10;

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    stock_library(&mut state, &reg, P0, 20);
    stock_library(&mut state, &reg, P1, 20);

    let mut cast = false;
    let mut actions = 0;
    engine::run_game_loop(&mut state, &reg, |_, player, _| {
        actions += 1;
        if actions > 30 {
            return mtg_engine::actions::Action::Concede;
        }
        if player == P0 && !cast {
            cast = true;
            return cast_action(bolt, vec![mtg_engine::actions::Target::Player(P1)]);
        }
        mtg_engine::actions::Action::PassPriority
    });

    assert_eq!(state.get_player(P1).life, 7,
        "10 less Lightning Bolt's 3 — 4 would mean it resolved twice");
}

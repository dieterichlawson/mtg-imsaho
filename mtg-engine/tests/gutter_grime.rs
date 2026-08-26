//! Tests for Gutter Grime — dynamic Ooze token P/T.
//!
//! Oracle: "Whenever a nontoken creature you control dies, put a slime counter on
//! Gutter Grime, then create a green Ooze creature token with 'This creature's
//! power and toughness are each equal to the number of slime counters on Gutter Grime.'"
//!
//! Key behaviors:
//! - Ooze token P/T dynamically tracks slime counter count on source Gutter Grime
//! - Adding more slime counters makes ALL existing Ooze tokens bigger
//! - Only nontoken creatures trigger it
//! - Only creatures you control trigger it

mod common;
use common::*;
use mtg_engine::triggers;
use mtg_engine::types::*;

/// A slime counter and an Ooze per nontoken creature death, and the Oozes are
/// sized by the *current* counter count — so an Ooze made when the count was 1
/// is a 2/2 once the count reaches 2. That is what "power and toughness are
/// each equal to the number of slime counters on Gutter Grime" means: a
/// characteristic-defining ability, recomputed, not a size fixed at creation.
#[test]
fn every_ooze_is_sized_by_the_current_slime_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    for expected in 1..=2 {
        let creature = ready_creature(&mut state, P0, 2, 2);
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, grime, CounterType::Slime), expected,
            "one slime counter per nontoken creature death");
        assert_eq!(count_tokens_named(&state, "Ooze"), expected as usize,
            "and one Ooze per death");

        // Every Ooze, including the ones made earlier, is the current size.
        let oozes: Vec<_> = state.objects.values()
            .filter(|o| o.is_token && o.zone == Zone::Battlefield && o.name == "Ooze")
            .map(|o| o.id)
            .collect();
        for ooze in oozes {
            assert_eq!(state.effective_power(ooze, &reg), Some(expected as i32),
                "with {expected} slime counter(s), every Ooze is {expected}/{expected}");
            assert_eq!(state.effective_toughness(ooze, &reg), Some(expected as i32));
        }
    }
}

/// "Whenever a **nontoken** creature **you control** dies" — two conditions,
/// and both need a row, since a Gutter Grime that ignored the condition
/// entirely would satisfy either one alone.
#[test]
fn gutter_grime_counts_only_your_own_nontoken_creatures() {
    // (whose creature, is it a token, does the slime counter arrive?)
    const CASES: &[(PlayerId, bool, bool)] = &[
        (P0, false, true),
        (P0, true, false),
        (P1, false, false),
    ];

    for &(controller, is_token, counts) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

        let creature = if is_token {
            let id = state.create_token("Spirit", controller, 1, 1,
                vec![Color::White], vec![CardType::Creature], vec![], &reg)[0];
            state.get_object_mut(id).unwrap().summoning_sick = false;
            id
        } else {
            ready_creature(&mut state, controller, 2, 2)
        };

        // Killed for real, so a dying token is removed from `state.objects` by
        // SBA 704.5d before the trigger resolves — the case where reading
        // `is_token` back off the dead object answers `false` and the "nontoken"
        // clause silently stops applying.
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, grime, CounterType::Slime), u32::from(counts),
            "controller=p{}, is_token={is_token}", controller.0);
        assert_eq!(count_tokens_named(&state, "Ooze"), usize::from(counts),
            "controller=p{}, is_token={is_token}", controller.0);
    }
}

/// The Oozes' size is read off Gutter Grime, so losing Gutter Grime makes them
/// 0/0 — and state-based actions then bury them (CR 704.5a).
#[test]
fn the_oozes_die_when_gutter_grime_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    kill_by_damage(&mut state, &reg, creature);
    triggers::process_triggers(&mut state, &reg);

    let ooze = find_token_named(&state, "Ooze").expect("an Ooze was made");
    assert_eq!(state.effective_power(ooze, &reg), Some(1), "test precondition: a 1/1");

    state.move_object(grime, Zone::Graveyard, &reg);

    assert_eq!(state.effective_power(ooze, &reg), Some(0),
        "with no Gutter Grime there are no slime counters to count");
    assert_eq!(state.effective_toughness(ooze, &reg), Some(0));

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert!(state.get_object(ooze).is_none(),
        "a 0-toughness token dies and, being a token, ceases to exist");
}

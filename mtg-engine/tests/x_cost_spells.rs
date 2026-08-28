//! Casting an {X} spell: the autotap planner, the funding prompt, and the X
//! the spell ends up with.
//!
//! CR 601.2b — X is chosen as the spell is put on the stack, and the cost is
//! then the announced X plus the printed remainder. The engine asks: the spell
//! stays in hand through a `ChooseXFunding` prompt and only reaches the stack
//! once the player has said which sources pay for it. That prompt is the whole
//! point, so a test that submits the cast and stops has not tested casting.
//!
//! These four cases used to be four near-identical tests, one of which ended
//! at `let _new_state = submit_action(...)` under the comment "should not
//! panic" — it would have passed with X computed as anything at all.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::engine;
use mtg_engine::types::*;

/// How the mana for the cast is available.
enum Mana {
    /// This many untapped lands of the given name — the autotap planner has
    /// to find them, which is what used to panic with "InsufficientMana"
    /// because an X spell's tap plan came back empty.
    Lands(&'static str, usize),
    /// Already floating, so nothing is tapped.
    Pool(ManaType, u32),
}

/// Cast `name` for the most X the board can fund, and return the resulting
/// state with the spell on the stack.
fn cast_for_max_x(
    reg: &mtg_engine::cards::CardRegistry,
    name: &str,
    mana: &Mana,
) -> (GameState, ObjectId) {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let spell = spell_in_hand(&mut state, reg, name, P0);
    match *mana {
        Mana::Lands(land, n) => for _ in 0..n { named_permanent(&mut state, reg, land, P0); },
        Mana::Pool(kind, n) => state.get_player_mut(P0).mana_pool.add(kind, n),
    }
    // Something to point at, so nothing is held back for want of a target.
    ready_creature(&mut state, P1, 5, 5);

    let legal = engine::legal_actions(&state, reg);
    let cast = legal.actions.iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell))
        .unwrap_or_else(|| panic!("{name} should be castable"));

    let state = engine::submit_action(&state, cast, reg);
    assert!(matches!(&state.awaiting_action, Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { .. }, .. })),
        "{name}: casting an X spell asks how X is funded before it reaches the \
         stack (CR 601.2b); got {:?}", state.awaiting_action);

    (resolve_funding_max(&state, reg), spell)
}

/// X is everything the board can pay beyond the printed part of the cost, and
/// the spell reaches the stack carrying it.
#[test]
fn x_is_what_is_left_after_the_printed_cost_is_paid() {
    let reg = registry();
    // (spell, how the mana is available, expected X)
    let cases: &[(&str, Mana, u32)] = &[
        // {X}{R}: three Mountains pay {R} and leave two.
        ("Devil's Play", Mana::Lands("Mountain", 3), 2),
        ("Devil's Play", Mana::Lands("Mountain", 5), 4),
        // Already floating: nothing to tap, same arithmetic.
        ("Devil's Play", Mana::Pool(ManaType::Red, 3), 2),
        // {X}{W} on a creature rather than a sorcery.
        ("Mikaeus, the Lunarch", Mana::Lands("Plains", 4), 3),
    ];

    for (name, mana, expected_x) in cases {
        let (state, spell) = cast_for_max_x(&reg, name, mana);
        let obj = state.get_object(spell).expect("the spell still exists");
        assert_eq!(obj.zone, Zone::Stack,
            "{name}: the spell is on the stack once funding is settled");
        assert_eq!(obj.x_value, Some(*expected_x),
            "{name}: X is the mana available beyond the printed cost");
    }
}

/// CR 107.3b: a permanent that was not cast has X = 0, and CR 400.7 makes a
/// permanent that leaves and comes back a new object — one that was never cast
/// at all. So a Mikaeus cast for X=3, killed, and reanimated arrives with no
/// counters, as a 0/0, and dies to state-based action.
///
/// `x_value` used to sit on the object through the graveyard, so the
/// enters-with-counters replacement read the *old* cast's X on the way back
/// in and the reanimated Mikaeus came back a 3/3.
#[test]
fn a_reanimated_x_creature_does_not_remember_the_x_it_was_cast_for() {
    let reg = registry();
    let (mut state, mikaeus) = cast_for_max_x(&reg, "Mikaeus, the Lunarch", &Mana::Lands("Plains", 4));
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_counter_count(mikaeus, CounterType::PlusOnePlusOne), 3, "test setup");

    // It dies...
    mtg_engine::destruction::try_destroy(&mut state, mikaeus, &reg);
    assert_eq!(state.get_object(mikaeus).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(mikaeus).unwrap().x_value, None,
        "the X went with the object it was chosen for");

    // ...and something puts it back.
    state.move_object(mikaeus, Zone::Battlefield, &reg);
    assert_eq!(state.get_counter_count(mikaeus, CounterType::PlusOnePlusOne), 0,
        "X is 0 for a permanent that was not cast (CR 107.3b)");
    assert_eq!(
        (state.effective_power(mikaeus, &reg), state.effective_toughness(mikaeus, &reg)),
        (Some(0), Some(0)),
        "so it is the printed 0/0");

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(mikaeus).unwrap().zone, Zone::Graveyard,
        "and a 0/0 does not stay (CR 704.5f)");
}

/// The X a spell was cast for is the X its effect uses. Devil's Play deals
/// that much damage; Mikaeus arrives with that many +1/+1 counters
/// (CR 107.3e — X in a resolving spell's text is the value chosen for it).
#[test]
fn the_announced_x_is_the_x_the_spell_resolves_with() {
    let reg = registry();

    let (mut state, spell) = cast_for_max_x(&reg, "Devil's Play", &Mana::Lands("Mountain", 5));
    let target = state.objects_in_zone(Zone::Battlefield, P1)[0].id;
    // The cast locked its target; resolving deals X to it.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(target).unwrap().damage_marked, 4,
        "Devil's Play cast for X=4 deals 4, not its printed 0");

    let (mut state, mikaeus) = cast_for_max_x(&reg, "Mikaeus, the Lunarch", &Mana::Lands("Plains", 4));
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    let obj = state.get_object(mikaeus).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert_eq!(state.get_counter_count(mikaeus, CounterType::PlusOnePlusOne), 3,
        "Mikaeus enters with X +1/+1 counters, and X was 3");
}

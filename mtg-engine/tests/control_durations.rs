//! "For as long as" durations end when their condition stops being true.
//!
//! CR 611.2b. Olivia Voldaren's "{3}{B}{B}: Gain control of target Vampire for
//! as long as you control Olivia Voldaren" tracked what it had stolen in the
//! card's own scratch state and unwound it from `on_leave_battlefield` — so
//! the effect ended in exactly one way. Take Olivia with an Act of Treason and
//! you no longer control her, the condition is false, and the stolen Vampires
//! must go home; but nothing had happened that the card was watching for.
//!
//! The duration is written down on the game state now and checked as a
//! state-based action, which is the closest the engine has to "the moment".

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Olivia plus a Vampire the opponent controls, already stolen.
fn olivia_with_a_stolen_vampire() -> (mtg_engine::state::GameState, mtg_engine::ids::ObjectId, mtg_engine::ids::ObjectId, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    assert!(state.has_subtype(vampire, "Vampire", &reg), "test precondition");

    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(vampire)]);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_object(vampire).unwrap().controller, P0,
        "test precondition: Olivia took the Vampire");

    (state, olivia, vampire, reg)
}

/// An opponent taking Olivia — no zone change at all — ends the effect.
#[test]
fn stolen_vampires_returned_when_olivia_control_changes_without_zone_change() {
    let (mut state, olivia, vampire, reg) = olivia_with_a_stolen_vampire();

    // Act of Treason on Olivia: P1 controls her now.
    state.change_control(olivia, P1);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(olivia).unwrap().zone, Zone::Battlefield,
        "Olivia is still on the battlefield — this is not a zone change");
    assert_eq!(state.get_object(vampire).unwrap().controller, P1,
        "P0 no longer controls Olivia, so 'for as long as you control Olivia' \
         is over and the Vampire goes back to its controller (CR 611.2b)");
}

/// The original way it ended still works.
#[test]
fn stolen_vampires_returned_when_olivia_leaves_the_battlefield() {
    let (mut state, olivia, vampire, reg) = olivia_with_a_stolen_vampire();

    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(vampire).unwrap().controller, P1);
}

/// While the condition holds, nothing happens — repeated SBA passes must not
/// hand the Vampire back on their own.
#[test]
fn the_effect_persists_while_its_condition_holds() {
    let (mut state, _olivia, vampire, reg) = olivia_with_a_stolen_vampire();

    for _ in 0..3 {
        mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    }

    assert_eq!(state.get_object(vampire).unwrap().controller, P0,
        "P0 still controls Olivia, so the effect is still on");
}

/// Several stolen creatures all go back together, and the bookkeeping is
/// cleared so a later Olivia doesn't inherit it.
#[test]
fn every_stolen_creature_goes_back_at_once() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let a = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    let b = named_permanent(&mut state, &reg, "Vampire Interloper", P1);

    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(a)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(b)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.control_effects.len(), 2, "two control effects in force");

    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(a).unwrap().controller, P1);
    assert_eq!(state.get_object(b).unwrap().controller, P1);
    assert!(state.control_effects.is_empty(),
        "the ended effects are gone from the state, not left to fire again");
}

/// A creature that has left the battlefield doesn't come back to be handed
/// over — the effect just ends.
#[test]
fn a_stolen_creature_that_died_is_simply_forgotten() {
    let (mut state, olivia, vampire, reg) = olivia_with_a_stolen_vampire();

    mtg_engine::destruction::try_destroy(&mut state, vampire, &reg);
    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(vampire).unwrap().zone, Zone::Graveyard);
    assert!(state.control_effects.is_empty());
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Traitorous Blood gives control "until end of turn" but the engine
/// never reverts the control change during cleanup.
/// Oracle: "Gain control of target creature until end of turn."
#[test]
fn bug_control_change_not_reverted_at_eot() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place a creature for P1
    let creature = ready_creature(&mut state, P1, 3, 3);
    assert_eq!(state.get_object(creature).unwrap().controller, P1);

    // Cast Traitorous Blood on it
    let spell = castable_spell(&mut state, &registry, "Traitorous Blood", P0);
    state = cast_and_resolve(&state, &registry, spell, vec![Target::Object(creature)]);

    // Creature should now be controlled by P0
    assert_eq!(state.get_object(creature).unwrap().controller, P0,
        "Traitorous Blood should give control to P0");

    // Run the game to the cleanup step. Replaying the cleanup step's body here
    // instead would assert only that the copy works — it would still pass with
    // the engine's cleanup deleted.
    advance_to_cleanup(&mut state, &registry);

    assert_eq!(state.get_object(creature).unwrap().controller, P1,
        "Control should revert to P1 at end of turn");
}

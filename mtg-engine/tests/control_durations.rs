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

/// Olivia's ability reads "target Vampire" — no "another", no "you don't
/// control" — so she is a legal target for her own ability, as is any Vampire
/// you already control. The effect changes nothing right now, and it is still
/// a real layer-2 effect with its own timestamp (CR 613.1b, 613.7a): it is
/// what keeps the permanent when an until-end-of-turn steal wears off.
///
/// The invariant checker used to call `controller == original_controller`
/// structurally impossible and abort the game on it (issue #286), which every
/// `--check-invariants` run over a decklist with an Olivia in it could hit.
#[test]
fn olivia_may_take_a_vampire_its_controller_already_controls() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let mine = named_permanent(&mut state, &reg, "Markov Patrician", P0);

    for target in [mine, olivia] {
        activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(target)]);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        assert_eq!(state.get_object(target).unwrap().controller, P0,
            "control does not visibly change");
        assert!(state.control_effects.iter().any(|c| c.object == target),
            "but the effect is recorded (CR 613.1b)");
        let complaints: Vec<String> = mtg_engine::invariants::check_settled(&state, &reg)
            .into_iter()
            .filter(|m| m.contains("control effect"))
            .collect();
        assert!(complaints.is_empty(),
            "a redundant control effect is legal, got {complaints:?}");
    }
}

/// Issue #285, the line that cost a player their own creature: p1's Olivia
/// steals p0's Vampire, p0 takes it back with Traitorous Blood, then kills
/// Olivia. Both effects have ended by the cleanup step, and with no
/// control-changing effect left the permanent is controlled by the player who
/// put it onto the battlefield (CR 110.2a, 613.1b).
#[test]
fn a_creature_stolen_back_and_then_freed_goes_home_not_to_the_thief() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);

    let interloper = named_permanent(&mut state, &reg, "Vampire Interloper", P0);
    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P1);

    // p1's Olivia takes it, "for as long as you control Olivia".
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(interloper)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_object(interloper).unwrap().controller, P1, "test precondition");

    // p0 takes their own creature back until end of turn.
    let blood = spell_in_hand(&mut state, &reg, "Traitorous Blood", P0);
    state.get_object_mut(blood).unwrap().controller = P0;
    reg.get(state.get_object(blood).unwrap().card_id).unwrap()
        .on_resolve(&mut state, blood, &[Target::Object(interloper)], &reg);
    assert_eq!(state.get_object(interloper).unwrap().controller, P0,
        "test precondition: p0 has it back");

    // Olivia dies. Her effect ends; the until-end-of-turn one has not.
    state.move_object(olivia, Zone::Graveyard, &reg);
    while mtg_engine::sba::check_state_based_actions(&mut state, &reg) {}
    assert_eq!(state.get_object(interloper).unwrap().controller, P0,
        "the later effect is still in force, so the creature stays with p0 (CR 613.7a)");

    // Cleanup ends the last effect. Nothing is left, so it is p0's creature.
    advance_to_cleanup(&mut state, &reg);
    assert_eq!(state.get_object(interloper).unwrap().controller, P0,
        "with no control effect left the permanent goes to its default \
         controller — the player who put it onto the battlefield (CR 110.2a)");
    assert!(state.control_effects.is_empty());
}

/// The mirror line, which #253 fixed and must keep working: the durable
/// effect is created *after* the temporary one, so it is the later timestamp
/// and it keeps the creature when the temporary one ends at cleanup.
#[test]
fn a_durable_effect_created_after_a_steal_keeps_the_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);

    // p0 steals it until end of turn, then points Olivia at it.
    let blood = spell_in_hand(&mut state, &reg, "Traitorous Blood", P0);
    reg.get(state.get_object(blood).unwrap().card_id).unwrap()
        .on_resolve(&mut state, blood, &[Target::Object(victim)], &reg);
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(victim)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    advance_to_cleanup(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().controller, P0,
        "Olivia's effect outlives the cleanup step and keeps the creature");
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

    // Issue #256: the steal was announced and the return was not, so a
    // permanent moved from one side of the board to the other with nothing
    // in the record. The other duration
    // (`expire_control_effects`) has always logged the same event.
    let name = state.obj_name(creature);
    assert!(
        state.game_log.iter().any(|e|
            e.level > mtg_engine::state::LogLevel::Private
            && e.message.contains(&name)
            && e.message.contains("returns to p1")),
        "the return is in the log a player can read:\n{:#?}",
        state.game_log.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

/// Issue #253: the durable half of the classic pair.
///
/// Traitorous Blood takes a creature "until end of turn"; Olivia's
/// "{3}{B}{B}: Gain control of target Vampire for as long as you control
/// Olivia Voldaren" is then pointed at the creature you now control.
/// Targeting a Vampire you already control is legal — "target Vampire" has
/// no "you don't control" clause — and the effect still applies, in layer 2,
/// with its own timestamp and its own duration (CR 613.1b, 613.7a, 611.2b).
/// When the temporary effect ends in the cleanup step the later one is still
/// there, so the creature stays.
///
/// The engine returned early whenever the target was already yours, so the
/// ability was a five-mana no-op that logged a steal which had not happened,
/// and the creature went home at cleanup.
#[test]
fn olivia_keeps_what_traitorous_blood_only_borrowed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = named_permanent(&mut state, &reg, "Vampire Interloper", P1);
    assert!(state.has_subtype(vampire, "Vampire", &reg), "test precondition");

    // Steal it until end of turn...
    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(vampire)]);
    assert_eq!(state.get_object(vampire).unwrap().controller, P0, "test precondition");

    // ...then point Olivia at the creature you now control.
    activate_via_hooks(&mut state, &reg, olivia, 1, &[Target::Object(vampire)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(
        state.control_effects.iter().any(|e| e.object == vampire && e.source == olivia),
        "the ability records its effect even when the target is already yours"
    );

    advance_to_cleanup(&mut state, &reg);
    assert_eq!(state.get_object(vampire).unwrap().controller, P0,
        "Traitorous Blood's effect ended; Olivia's did not, so the Vampire stays");

    // And it is Olivia's effect holding it: lose her and it goes home — to
    // its owner, not to the player Traitorous Blood took it from for a turn.
    state.move_object(olivia, Zone::Graveyard, &reg);
    state.expire_control_effects();
    assert_eq!(state.get_object(vampire).unwrap().controller, P1,
        "with no control effect left, the permanent is its owner's again (CR 110.2)");
}

/// The same shape without Olivia: a plain "until end of turn" steal still
/// goes home, which is what the durable effect above has to be distinguished
/// from rather than break.
#[test]
fn a_borrowed_creature_with_no_durable_effect_still_goes_home() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let vampire = named_permanent(&mut state, &reg, "Vampire Interloper", P1);
    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(vampire)]);
    assert_eq!(state.get_object(vampire).unwrap().controller, P0);

    advance_to_cleanup(&mut state, &reg);
    assert_eq!(state.get_object(vampire).unwrap().controller, P1);
}

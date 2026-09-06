//! CR 614.12b / 706.9a: "you may have this enter as a copy of any creature on
//! the battlefield" is a choice made *as the permanent enters*, so everything
//! that follows from the copy — its copiable values, the replacement effects
//! printed on the copied card, the abilities that trigger on it entering —
//! happens as part of entering, with no window in between.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::state::{AwaitingAction, EnterAsCopyChoice, ResolutionChoiceKind};
use mtg_engine::types::*;

/// The consequence the old ETB-trigger implementation could not reach: the
/// copied card's own "enters tapped" replacement (CR 614.1c) applies, because
/// the permanent already *is* that card as it enters.
#[test]
fn a_copy_of_a_creature_that_enters_tapped_enters_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P1);

    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(grimgrin));

    assert_eq!(state.name_of(twin, &reg), "Grimgrin, Corpse-Born");
    assert!(state.get_object(twin).unwrap().tapped,
        "the copied card's 'enters tapped' applies to the copy (CR 614.1c)");
}

/// A declined copy is a printed 0/0 and enters untapped — the same card,
/// the other answer.
#[test]
fn a_declined_copy_enters_as_the_printed_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P1);

    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, None);

    assert_eq!(state.name_of(twin, &reg), "Evil Twin");
    assert!(!state.get_object(twin).unwrap().tapped);
    assert_eq!(state.effective_toughness(twin, &reg), Some(0),
        "the printed body is a 0/0 (CR 704.5f will have it)");
}

/// CR 706.2: the copiable values of a permanent that is itself a copy are the
/// values it copied. Copying a copy therefore reaches the original card, not
/// the printed card underneath it.
#[test]
fn copying_a_copy_takes_what_that_copy_copied() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let first = enters_as_copy_of(&mut state, &reg, "Evil Twin", P1, Some(bear));
    assert_eq!(state.name_of(first, &reg), "Grizzly Bears", "test precondition");

    let second = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(first));

    assert_eq!(state.name_of(second, &reg), "Grizzly Bears",
        "a copy of a copy is a copy of what the first one copied (CR 706.2)");
    assert_eq!(state.effective_power(second, &reg), Some(2));
}

/// The choice is not tied to casting: a permanent put onto the battlefield
/// from anywhere (Grimoire of the Dead reanimating an Evil Twin) is asked
/// too, and does not arrive until it is answered.
#[test]
fn an_evil_twin_reanimated_from_the_graveyard_is_asked_as_well() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card_id = reg.get_id_by_name("Evil Twin").unwrap();
    let twin = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(twin).unwrap().name = "Evil Twin".into();

    state.move_object(twin, Zone::Battlefield, &reg);
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "reanimation waits on the choice too");
    mtg_engine::replacement::process_pending_entry_choices(&mut state, &reg);
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player,
        choice: ResolutionChoiceKind::ChooseTarget { options, .. },
        ..
    }) if *player == P0 && options.contains(&Target::Object(bear)));
    assert!(asked, "got {:?}", state.awaiting_action);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bear))) },
        &reg,
    );
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.name_of(twin, &reg), "Grizzly Bears");
}

/// Two of them entering at once each get their own answer — one prompt at a
/// time, and neither entry happens before its own choice is made.
#[test]
fn two_entering_copies_are_asked_one_at_a_time() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card_id = reg.get_id_by_name("Evil Twin").unwrap();
    let a = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    let b = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    for id in [a, b] {
        state.get_object_mut(id).unwrap().name = "Evil Twin".into();
        state.move_object(id, Zone::Battlefield, &reg);
    }

    assert_eq!(state.pending_entry_choices, vec![a, b], "both are queued, neither entered");

    let mut state = state;
    for id in [a, b] {
        mtg_engine::replacement::process_pending_entry_choices(&mut state, &reg);
        assert!(state.awaiting_action.is_some(), "the queue asks for #{}", id.0);
        state = mtg_engine::engine::submit_action(
            &state,
            &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bear))) },
            &reg,
        );
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    }
    assert!(state.pending_entry_choices.is_empty());
    assert_eq!(state.name_of(a, &reg), "Grizzly Bears");
    assert_eq!(state.name_of(b, &reg), "Grizzly Bears");
}

/// CR 400.7: a permanent that leaves the battlefield is a new object, and a
/// new object chooses again. The recorded answer must not follow the card
/// into the graveyard and back.
#[test]
fn the_recorded_choice_does_not_survive_a_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(bear));

    state.move_object(twin, Zone::Graveyard, &reg);

    assert_eq!(state.get_object(twin).unwrap().entering_copy_choice, EnterAsCopyChoice::Unasked,
        "the answer is forgotten with the object it was made for");
    state.move_object(twin, Zone::Battlefield, &reg);
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "so coming back asks again rather than silently re-using the old answer");
}

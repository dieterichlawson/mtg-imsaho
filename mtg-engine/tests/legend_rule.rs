//! CR 704.5j, the legend rule, from two sides the rulebook sweep found
//! wanting: which permanents count as legendary, and what happens to the
//! one that is not kept.

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::events::GameEvent;
use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
use mtg_engine::types::*;

/// CR 707.2: supertypes are copiable values. A legendary creature that enters
/// as a copy of Essence of the Wild is an Essence of the Wild, and Essence is
/// not legendary — the flag used to be stamped from the card that was cast.
#[test]
fn a_legend_entering_as_a_copy_of_a_non_legend_is_not_legendary() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Essence of the Wild", P0);
    let olivia = castable_spell(&mut state, &reg, "Olivia Voldaren", P0);

    let state = cast_and_resolve(&state, &reg, olivia, vec![]);

    assert_eq!(state.name_of(olivia, &reg), "Essence of the Wild", "test precondition: it entered as a copy");
    assert!(!state.get_object(olivia).unwrap().is_legendary);
    assert!(!state.is_legendary(olivia, &reg),
        "a copy of a non-legendary card is not subject to the legend rule (CR 707.2)");
}

/// CR 700.4: put into a graveyard from the battlefield is "dies", the legend
/// rule included. The unkept legend used to leave by a bare zone move, so
/// morbid and every "whenever a creature dies" watcher missed it.
#[test]
fn the_legend_rule_loser_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let first = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    let second = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    for id in [first, second] {
        state.get_object_mut(id).unwrap().name = "Geist of Saint Traft".into();
    }

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseTarget { effect: PendingEffect::LegendRuleKeep { .. }, .. }, ..
    }) = &state.awaiting_action else {
        panic!("expected the legend-rule keep prompt, got {:?}", state.awaiting_action);
    };

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(first))) },
        &reg,
    );

    assert_eq!(state.get_object(second).unwrap().zone, Zone::Graveyard);
    assert!(state.events.iter().any(|e| matches!(e, GameEvent::CreatureDied { object, .. } if *object == second)),
        "the unkept legend died (CR 700.4): {:?}", state.events);
    assert!(state.creature_died_this_turn, "morbid saw it");
}

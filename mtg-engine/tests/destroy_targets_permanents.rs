//! CR 701.7a: "destroy" moves a permanent from the battlefield to its
//! owner's graveyard. A card that is already in the graveyard is not a
//! permanent — and, being a new object (CR 400.7), it is not the creature an
//! earlier "destroy that creature" was about. Found by fuzzing: Creepy
//! Doll's combat-damage trigger resolving after first-strike damage had
//! already killed Falkenrath Noble announced the Noble's death a second time.

mod common;
use common::*;
use mtg_engine::destruction::{try_destroy, try_destroy_all, try_destroy_by, DestroyResult};
use mtg_engine::events::GameEvent;
use mtg_engine::types::*;

#[test]
fn destroying_a_creature_that_already_died_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);
    kill_by_damage(&mut state, &reg, noble);
    assert_eq!(state.get_object(noble).unwrap().zone, Zone::Graveyard, "test precondition");
    state.events.clear();
    state.creature_died_this_turn = false;

    assert_eq!(try_destroy(&mut state, noble, &reg), DestroyResult::NotAPermanent);
    assert_eq!(try_destroy_by(&mut state, noble, "Creepy Doll", &reg), DestroyResult::NotAPermanent);

    assert!(!state.events.iter().any(|e| matches!(e, GameEvent::CreatureDied { .. })),
        "no second death is announced (CR 400.7): {:?}", state.events);
    assert!(!state.creature_died_this_turn, "and morbid does not see one");
    assert_eq!(state.get_object(noble).unwrap().zone, Zone::Graveyard);
}

#[test]
fn a_simultaneous_destruction_skips_what_is_not_on_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dead = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let alive = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(dead, Zone::Graveyard, &reg);
    state.events.clear();

    let results = try_destroy_all(&mut state, &[dead, alive], &reg);

    assert_eq!(results, vec![(dead, DestroyResult::NotAPermanent), (alive, DestroyResult::Died)]);
    assert_eq!(state.events.iter().filter(|e| matches!(e, GameEvent::CreatureDied { .. })).count(), 1,
        "exactly one creature died: {:?}", state.events);
}

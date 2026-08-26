//! A permanent that enters under someone else's control must already be under
//! that control when `EnteredBattlefield` fires (CR 110.2).
//!
//! Reanimation and exile-return cards used to call `move_object` and assign
//! `controller` on the next line. `move_object` emits the event during the
//! move, reading the controller as it stands at that moment — so the object
//! ended up right but the event carried the *previous* controller, and every
//! `AnyCreatureEnters` watcher that reads `entered_controller` saw the wrong
//! player. `move_object_under_control` sets it first.
//!
//! The stale value is not hypothetical: `move_object` deliberately does not
//! reset `controller` to `owner` on leaving the battlefield (death triggers
//! rely on it as last known information), so a creature that died while
//! stolen sits in its owner's graveyard still marked as the thief's.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;
/// The controller carried by the `EnteredBattlefield` event for `object`.
fn entered_controller(state: &GameState, object: ObjectId) -> Option<PlayerId> {
    state.events.iter().rev().find_map(|e| match e {
        GameEvent::EnteredBattlefield { object: o, controller } if *o == object => Some(*controller),
        _ => None,
    })
}

/// A creature card in P1's graveyard still marked as controlled by P0 — what
/// a creature that died while stolen actually looks like.
fn stolen_corpse(state: &mut GameState, reg: &CardRegistry, name: &str) -> ObjectId {
    let id = named_card_in_graveyard(state, reg, name, P1);
    state.get_object_mut(id).unwrap().controller = P0;
    id
}

#[test]
fn move_object_under_control_emits_the_new_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let corpse = stolen_corpse(&mut state, &reg, "Walking Corpse");
    assert_eq!(state.get_object(corpse).unwrap().controller, P0,
        "test precondition: the card carries a stale controller");

    state.events.clear();
    state.move_object_under_control(corpse, Zone::Battlefield, P1, &reg);

    assert_eq!(entered_controller(&state, corpse), Some(P1),
        "the EnteredBattlefield event must carry the controller the permanent \
         is entering under, not the one it happened to have in the graveyard");
    assert_eq!(state.get_object(corpse).unwrap().controller, P1);
}

/// The plain `move_object` keeps its existing behaviour — it enters under
/// whatever controller the object already has.
#[test]
fn plain_move_object_is_unchanged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);

    state.events.clear();
    state.move_object(corpse, Zone::Battlefield, &reg);

    assert_eq!(entered_controller(&state, corpse), Some(P1));
}

/// Fiend Hunter returns the exiled creature under its OWNER's control, even
/// if it was stolen when Fiend Hunter exiled it.
#[test]
fn fiend_hunter_returns_the_creature_to_its_owner() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P0);
    let victim = named_permanent(&mut state, &reg, "Walking Corpse", P1);

    // Fiend Hunter exiles it and remembers it.
    state.move_object(victim, Zone::Exile, &reg);
    state.get_object_mut(hunter).unwrap()
        .card_state.insert("exiled_creature".into(), victim);
    // While exiled it still carries a stale controller from a steal effect.
    state.get_object_mut(victim).unwrap().controller = P0;

    state.events.clear();
    let behavior = reg.get(state.get_object(hunter).unwrap().card_id).unwrap();
    behavior.on_leave_battlefield(&mut state, hunter, &reg);

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "the exiled creature should return");
    assert_eq!(state.get_object(victim).unwrap().controller, P1,
        "it returns under its owner's control (CR 110.2)");
    assert_eq!(entered_controller(&state, victim), Some(P1),
        "and the EnteredBattlefield event must say so, since watchers read it");
}

/// Splinterfright's "power and toughness equal to the number of creature
/// cards in your graveyard" counts its OWNER's graveyard. `objects_in_zone`
/// filters graveyards by owner, so reading a stale `controller` counted the
/// opponent's cards instead (CR 112.8).
#[test]
fn splinterfright_counts_its_owners_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fright = named_permanent(&mut state, &reg, "Splinterfright", P0);
    // Two creature cards in P0's graveyard, none in P1's.
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &reg, "Avacyn's Pilgrim", P0);

    let behavior = reg.get(state.get_object(fright).unwrap().card_id).unwrap();
    let before = behavior.dynamic_pt(&state, fright, &reg);

    // An Act-of-Treason style steal leaves a stale controller behind.
    state.get_object_mut(fright).unwrap().controller = P1;
    let after = behavior.dynamic_pt(&state, fright, &reg);

    assert_eq!(after, before,
        "Splinterfright counts its owner's graveyard, so a control change \
         must not change its size (was {before:?}, now {after:?})");
}

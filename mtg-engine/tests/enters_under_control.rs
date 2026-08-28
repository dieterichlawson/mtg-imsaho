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

/// CR 109.5: "you" on an object is its controller, and for a static ability
/// that is the *current* controller of the object it is on. Splinterfright's
/// "power and toughness are each equal to the number of creature cards in your
/// graveyard" is a characteristic-defining ability, which is a static ability
/// (CR 604.3) — so a stolen Splinterfright is the size of the thief's
/// graveyard.
///
/// This used to assert the opposite, on the reasoning that "an Act-of-Treason
/// style steal leaves a stale controller behind". It does not: a steal sets a
/// real controller, and the field the card was avoiding was the right one all
/// along. What makes `controller` correct in *both* zones is CR 108.4 — a card
/// off the battlefield has no controller, and the zone change resets
/// `controller` to `owner` on the way out.
#[test]
fn splinterfright_is_the_size_of_its_controllers_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fright = named_permanent(&mut state, &reg, "Splinterfright", P0);
    // Two creature cards in P0's graveyard, three in P1's.
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &reg, "Avacyn's Pilgrim", P0);
    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);
    }

    assert_eq!(state.effective_power(fright, &reg), Some(2),
        "under its owner, it counts their two");

    // An opponent takes it.
    state.get_object_mut(fright).unwrap().controller = P1;
    assert_eq!(state.effective_power(fright, &reg), Some(3),
        "under the thief, \"your graveyard\" is the thief's (CR 109.5)");
}

/// Ruling 2025-01-24: "The ability that defines Splinterfright's power and
/// toughness works in all zones, not just the battlefield. If Splinterfright
/// is in your graveyard, it will count itself."
///
/// A card in a graveyard has no controller, so "you" falls back to its owner
/// (CR 108.4 and the last clause of CR 109.5) — which is what the zone change
/// leaves in `controller`.
#[test]
fn splinterfright_in_a_graveyard_counts_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fright = named_permanent(&mut state, &reg, "Splinterfright", P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert_eq!(state.effective_power(fright, &reg), Some(1), "test setup");

    // It dies, and is now itself one of the creature cards it counts.
    state.move_object(fright, Zone::Graveyard, &reg);
    assert_eq!(state.effective_power(fright, &reg), Some(2),
        "the ability works in the graveyard too, and it counts itself");

    // Even after a steal: it lost its controller on the way out, so "you" is
    // its owner again.
    assert_eq!(state.get_object(fright).unwrap().controller, P0,
        "leaving the battlefield reset the controller to the owner (CR 108.4)");
}

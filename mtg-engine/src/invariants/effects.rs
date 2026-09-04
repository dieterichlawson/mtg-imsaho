//! Continuous effects the engine tracks on the game state: until-end-of-turn
//! effects and "for as long as" control effects (CR 611, 400.7). Settled
//! tier: a resolution may create an effect and move its target in either
//! order, and expiry is a state-based action.

use super::{player_ok, Violations};
use crate::cards::CardRegistry;
use crate::state::{until_eot_object_target, GameState, StackEntry, TemporaryEffect};
use crate::triggers::TriggerEvent;
use crate::types::{CardType, Zone};

pub(super) fn check_core(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    for e in &state.until_end_of_turn {
        // CR 611.2c: every until-end-of-turn effect on an object in this pool
        // is written for a creature; creature-ness never changes here.
        if let Some(t) = until_eot_object_target(e) {
            if state.get_object(t).is_some_and(|o| o.zone == Zone::Battlefield) && !state.is_creature(t, registry) {
                v.push(format!("until-end-of-turn effect on #{} which is no creature", t.0));
            }
        }
        // CR 702.34a: granted flashback is the card's own mana cost, on an
        // instant or sorcery card.
        if let TemporaryEffect::GrantFlashback { target, cost } = e {
            match state.get_object(*target) {
                None => v.push(format!("flashback granted to #{} which does not exist", target.0)),
                Some(o) => {
                    if o.is_token {
                        v.push(format!("flashback granted to token #{}", target.0));
                    }
                    if !state.has_card_type(*target, CardType::Instant, registry)
                        && !state.has_card_type(*target, CardType::Sorcery, registry)
                    {
                        v.push(format!("flashback granted to #{} ({}) which is no instant or sorcery (CR 702.34a)", target.0, o.name));
                    }
                    let printed = state.face_data(*target, registry).and_then(|d| d.cost);
                    if printed.as_ref() != Some(cost) {
                        v.push(format!("flashback granted to #{} ({}) for {cost:?} but its cost is {printed:?}", target.0, o.name));
                    }
                }
            }
        }
    }
    for c in &state.control_effects {
        if state.get_object(c.object).is_some_and(|o| o.zone == Zone::Battlefield) && !state.is_creature(c.object, registry) {
            v.push(format!("control effect over #{} which is no creature", c.object.0));
        }
    }
    // CR 302.6: control taken this turn (the effect ends at cleanup, so it
    // began after this turn's untap step) leaves the permanent summoning
    // sick under its new controller.
    for e in &state.until_end_of_turn {
        if let TemporaryEffect::ChangeControl { target, original_controller } = e {
            if let Some(o) = state.get_object(*target) {
                if o.zone == Zone::Battlefield && o.controller != *original_controller && !o.summoning_sick {
                    v.push(format!("#{} ({}) was taken from p{} this turn but is not summoning sick (CR 302.6)", target.0, o.name, original_controller.0));
                }
            }
        }
    }
    // CR 603.7d: the delayed exiles in this pool are for tokens.
    let delayed = state.end_of_combat_exiles.iter().map(|e| e.target_id)
        .chain(state.stack.iter().filter_map(|e| match e {
            StackEntry::Trigger(t) => match t.event { TriggerEvent::DelayedTokenExile { target_id } => Some(target_id), _ => None },
            _ => None,
        }));
    for t in delayed {
        if state.get_object(t).is_some_and(|o| !o.is_token) {
            v.push(format!("delayed exile of #{} which is a card, not a token", t.0));
        }
    }
}

pub(super) fn check_settled(state: &GameState, _registry: &CardRegistry, v: &mut Violations) {
    // CR 611.2c/400.7: an effect on a permanent is on a permanent that is
    // there. (`move_object` prunes these the moment the target leaves.)
    for e in &state.until_end_of_turn {
        if let Some(t) = until_eot_object_target(e) {
            if !state.get_object(t).is_some_and(|o| o.zone == Zone::Battlefield) {
                v.push(format!("until-end-of-turn effect {e:?} on #{} which is not on the battlefield (CR 400.7)", t.0));
            }
        }
        if let TemporaryEffect::GrantFlashback { target, .. } = e {
            if state.get_object(*target).is_none() {
                v.push(format!("flashback granted to missing #{}", target.0));
            }
        }
    }
    // CR 611.2b: a "for as long as you control" effect holds its condition
    // once SBAs have settled, and is about permanents that are there.
    for c in &state.control_effects {
        let what = format!("control effect over #{}", c.object.0);
        if c.controller != c.source_controller || c.controller == c.original_controller {
            v.push(format!("{what}: p{} took it via p{}'s source from p{}", c.controller.0, c.source_controller.0, c.original_controller.0));
        }
        if !player_ok(state, c.controller) || !player_ok(state, c.original_controller) {
            continue; // reported by the core check
        }
        match state.get_object(c.source) {
            Some(s) if s.zone == Zone::Battlefield && s.controller == c.source_controller => {}
            _ => v.push(format!("{what} survives its source #{} leaving p{}'s control (CR 611.2b)", c.source.0, c.source_controller.0)),
        }
        if !state.get_object(c.object).is_some_and(|o| o.zone == Zone::Battlefield) {
            v.push(format!("{what} survives the object leaving the battlefield (CR 400.7)"));
        }
    }
}

//! Continuous effects the engine tracks on the game state: until-end-of-turn
//! effects and "for as long as" control effects (CR 611, 400.7). Settled
//! tier: a resolution may create an effect and move its target in either
//! order, and expiry is a state-based action.

use super::{player_ok, Violations};
use crate::cards::CardRegistry;
use crate::state::{until_eot_object_target, GameState, TemporaryEffect};
use crate::types::Zone;

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

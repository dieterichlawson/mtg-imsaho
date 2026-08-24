//! Actions that need no ceremony: priority, lands, discards, conceding.

use super::super::Applied;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::types::Zone;
use super::super::*;

pub(crate) fn pass_priority(state: &mut GameState, _registry: &CardRegistry) -> Applied {
        let player = state.priority_player.unwrap_or(state.active_player);
        state.events.push(GameEvent::PriorityPassed { player });
        state.log(LogLevel::Debug, format!("p{} passes priority", player.0));
        state.consecutive_passes += 1;
    Applied::Continue
}

pub(crate) fn play_land(state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) -> Applied {
        let player = state.priority_player.expect("PlayLand requires priority");
        state.move_object(object_id, Zone::Battlefield, registry);
        // Remove from library order if somehow there (shouldn't be, it's in hand).
        state.get_player_mut(player).land_plays_remaining -= 1;
        state.events.push(GameEvent::LandPlayed {
            player,
            object: object_id,
        });
        // EnteredBattlefield is now emitted by move_object.
        // Lands don't have summoning sickness (only creatures care).
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.summoning_sick = false;
        }
        let name = card_name(&state, registry, object_id);
        state.log(LogLevel::Info, format!("p{} played {}", player.0, name));
        state.consecutive_passes = 0;
    Applied::Continue
}

pub(crate) fn discard_cards(state: &mut GameState, cards: &[ObjectId], registry: &CardRegistry) -> Applied {
        let is_hand_size = matches!(&state.awaiting_action,
            Some(AwaitingAction::DiscardToHandSize { .. }));
        let player = match &state.awaiting_action {
            Some(AwaitingAction::DiscardToHandSize { player, .. }) => *player,
            _ => state.active_player,
        };
        let names: Vec<String> = cards.iter()
            .map(|&id| card_name(&state, registry, id))
            .collect();
        for &card_id in cards {
            state.discard_card(card_id, registry);
        }
        if is_hand_size {
            state.log(LogLevel::Event,
                format!("p{} discarded {} (cleanup)", player.0, names.join(", ")));
        } else {
            for name in &names {
                state.log(LogLevel::Event, format!("p{} discarded {}", player.0, name));
            }
        }
        state.awaiting_action = None;
    Applied::Continue
}

pub(crate) fn concede(state: &mut GameState, _registry: &CardRegistry) -> Applied {
        if let Some(player) = state.priority_player {
            state.log(LogLevel::Milestone, format!("p{} concedes", player.0));
            state.get_player_mut(player).lost = true;
            state.events.push(GameEvent::PlayerLost {
                player,
                reason: crate::events::LossReason::Conceded,
            });
        }
    Applied::Continue
}

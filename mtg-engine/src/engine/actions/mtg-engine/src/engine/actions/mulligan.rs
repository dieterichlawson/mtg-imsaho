//! The mulligan phase: keeping, mulliganing, and bottoming.

use super::super::Applied;
use crate::cards::SacrificeCost;
use crate::actions::{Action, Target};
use crate::cards::CardRegistry;
use crate::combat;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::stack;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::triggers;
use crate::types::{Zone, CardType, Supertype, ManaCost, ManaSymbol, ContinuousEffect, Keyword, CounterType, Step, Color};
use super::super::*;

pub(crate) fn mulligan_keep(state: &mut GameState, registry: &CardRegistry) -> Applied {
        let player = match &state.awaiting_action {
            Some(AwaitingAction::MulliganDecision { player }) => *player,
            _ => panic!("MulliganKeep without MulliganDecision awaiting"),
        };
        let mull_count = state.get_player(player).mulligan_count;
        state.log(LogLevel::Event,
            format!("p{} keeps ({} mulligan{})", player.0, mull_count,
                if mull_count == 1 { "" } else { "s" }));
        state.get_player_mut(player).mulligan_kept = true;
        // Record this player's bottom obligation now so it'll be drained
        // once every player has finished the keep/mull sub-phase.
        state.pending_mulligan_bottoms.push((player, mull_count as usize));
        // Advance the within-round position past this player.
        state.mulligan_round_position += 1;
        advance_mulligan_phase(&mut *state, registry);
        state.consecutive_passes = 0;
    Applied::Continue
}

pub(crate) fn mulligan_mull(state: &mut GameState, registry: &CardRegistry) -> Applied {
        let player = match &state.awaiting_action {
            Some(AwaitingAction::MulliganDecision { player }) => *player,
            _ => panic!("MulliganMull without MulliganDecision awaiting"),
        };
        assert!(state.get_player(player).mulligan_count < crate::state::LONDON_MULLIGAN_CAP, "MulliganMull attempted after reaching the mulligan cap");
        // Put the entire hand on the bottom of the library (temporarily;
        // it will be shuffled immediately) and draw seven fresh cards.
        let hand_ids: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, player)
            .iter().map(|o| o.id).collect();
        for id in &hand_ids {
            state.move_object(*id, Zone::Library, registry);
            // Append to the end of the library_order (bottom).
            let lib = &mut state.get_player_mut(player).library_order;
            if !lib.contains(id) {
                lib.push(*id);
            }
        }
        // Shuffle.
        let mut rng = rand::thread_rng();
        state.get_player_mut(player).library_order.shuffle(&mut rng);
        // Redraw seven.
        let _ = draw_cards(&mut *state, player, 7, registry);
        state.get_player_mut(player).mulligan_count += 1;
        let mull_count = state.get_player(player).mulligan_count;
        state.log(LogLevel::Event,
            format!("p{} mulligans to {}", player.0, 7 - i32::try_from(mull_count).unwrap_or(i32::MAX)));
        // Mark that this round had a mulligan, then advance past this
        // player. The next player in turn order (who hasn't already kept)
        // will be asked. The mulled player will be re-asked next round.
        state.mulligan_round_mulled = true;
        state.mulligan_round_position += 1;
        advance_mulligan_phase(&mut *state, registry);
        state.consecutive_passes = 0;
    Applied::Continue
}

pub(crate) fn bottom_cards(state: &mut GameState, cards: &[ObjectId], registry: &CardRegistry) -> Applied {
        let (player, count) = match &state.awaiting_action {
            Some(AwaitingAction::BottomAfterMulligan { player, count }) => (*player, *count),
            _ => panic!("BottomCards without BottomAfterMulligan awaiting"),
        };
        assert_eq!(cards.len(), count,
            "BottomCards: expected {} cards, got {}", count, cards.len());
        // Validate the chosen cards are all in this player's hand and distinct.
        let hand_ids: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, player)
            .iter().map(|o| o.id).collect();
        let mut seen = std::collections::HashSet::new();
        for id in cards {
            assert!(hand_ids.contains(id),
                "BottomCards: card {:?} not in p{}'s hand", id, player.0);
            assert!(seen.insert(*id),
                "BottomCards: duplicate card {id:?}");
        }
        // Move each card from hand to library and append to the bottom, in
        // the order given (so `cards[0]` ends up bottom-most of the group).
        for &card_id in cards {
            state.move_object(card_id, Zone::Library, registry);
            let lib = &mut state.get_player_mut(player).library_order;
            lib.retain(|&id| id != card_id);
            lib.push(card_id);
        }
        // Bottoming is a hidden action — the opponent must not see which
        // specific cards were sent to the bottom. Log only the count.
        state.log(LogLevel::Event,
            format!("p{} bottomed {} card{}", player.0, count,
                if count == 1 { "" } else { "s" }));
        state.awaiting_action = None;
        advance_mulligan_phase(&mut *state, registry);
        state.consecutive_passes = 0;
    Applied::Continue
}

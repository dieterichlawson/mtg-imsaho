use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Liliana of the Veil {1}{B}{B} Legendary Planeswalker — Liliana (3 loyalty).
/// +1: Each player discards a card.
/// -2: Target player sacrifices a creature.
/// -6: Separate all permanents target player controls into two piles. That player sacrifices
///     all permanents in the pile of their choice.
pub struct LilianaOfTheVeil;

impl CardBehavior for LilianaOfTheVeil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Liliana of the Veil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Planeswalker],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Liliana".into()],
            power: None,
            toughness: None,
            oracle_text: "+1: Each player discards a card.\n−2: Target player sacrifices a creature.\n−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn starting_loyalty(&self) -> Option<u32> {
        Some(3)
    }

    fn loyalty_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<LoyaltyAbilityDef> {
        vec![
            LoyaltyAbilityDef {
                ability_index: 0,
                loyalty_change: 1,
                description: "+1: Each player discards a card".into(),
                target_requirement: None,
            },
            LoyaltyAbilityDef {
                ability_index: 1,
                loyalty_change: -2,
                description: "-2: Target player sacrifices a creature".into(),
                target_requirement: Some(TargetRequirement::PlayerOnly),
            },
            LoyaltyAbilityDef {
                ability_index: 2,
                loyalty_change: -6,
                description: "-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice".into(),
                target_requirement: Some(TargetRequirement::PlayerOnly),
            },
        ]
    }

    fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };

        match ability_index {
            0 => {
                // +1: Each player discards a card.
                // Per ruling: active player chooses first, then each other player in turn order.
                // Present ChooseCardFromHand to the active player first.
                // Use card_state to track which players still need to discard.
                let active = state.active_player;

                // Build list of players who need to discard, starting with active player.
                let mut players_to_discard: Vec<PlayerId> = vec![];
                // Active player first.
                if !state.get_player(active).lost {
                    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, active)
                        .iter().map(|o| o.id).collect();
                    if !hand.is_empty() {
                        players_to_discard.push(active);
                    }
                }
                // Then other players in turn order.
                for pid_idx in 0..state.players.len() {
                    let pid = PlayerId(pid_idx as u8);
                    if pid == active { continue; }
                    if state.get_player(pid).lost { continue; }
                    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, pid)
                        .iter().map(|o| o.id).collect();
                    if !hand.is_empty() {
                        players_to_discard.push(pid);
                    }
                }

                if players_to_discard.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        "Liliana +1: no player has cards to discard".into());
                    return;
                }

                // Store remaining players in card_state for chaining.
                // We'll process them one at a time via on_discard_choice.
                let first_player = players_to_discard[0];
                let remaining: Vec<PlayerId> = players_to_discard[1..].to_vec();

                // Store remaining player IDs in card_state as a comma-separated string.
                if let Some(obj) = state.get_object_mut(self_id) {
                    let remaining_str = remaining.iter()
                        .map(|p| p.0.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    obj.card_state.insert("liliana_discard_remaining".into(),
                        crate::ids::ObjectId(remaining_str.parse::<u64>().unwrap_or(0)));
                    // Use a more robust encoding: store count and each player ID.
                    // Since card_state maps String->ObjectId, encode as separate keys.
                    obj.card_state.insert("liliana_discard_count".into(),
                        crate::ids::ObjectId(remaining.len() as u64));
                    for (i, pid) in remaining.iter().enumerate() {
                        obj.card_state.insert(format!("liliana_discard_{}", i),
                            crate::ids::ObjectId(pid.0 as u64));
                    }
                }

                let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, first_player)
                    .iter().map(|o| o.id).collect();

                if hand.len() == 1 {
                    // Only one card — auto-discard.
                    let card_id = hand[0];
                    let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
                    state.move_object(card_id, Zone::Graveyard);
                    state.events.push(crate::events::GameEvent::Discarded {
                        player: first_player,
                        object: card_id,
                    });
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana +1: p{} discarded {}", first_player.0, name));
                    // Chain to next player.
                    self.chain_next_discard(state, self_id);
                } else {
                    // Present choice to the first player.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: first_player,
                        source: self_id,
                        choice: ResolutionChoiceKind::ChooseCardFromHand {
                            description: "Liliana +1: choose a card to discard".into(),
                            player: first_player,
                            cards: hand,
                        },
                    });
                }
            }
            1 => {
                // -2: Target player sacrifices a creature.
                let target_player = match targets.first() {
                    Some(Target::Player(pid)) => *pid,
                    _ => return,
                };

                let creatures: Vec<Target> = crate::cards::helpers::creatures_controlled_by(state, target_player);

                if creatures.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -2: p{} has no creatures to sacrifice", target_player.0));
                    return;
                }

                // The targeted player chooses which creature to sacrifice.
                crate::cards::helpers::present_target_choice(
                    state,
                    self_id,
                    target_player,
                    creatures,
                    PendingEffect::SacrificeCreature {
                        source_name: "Liliana of the Veil".into(),
                    },
                    "Liliana -2: choose a creature to sacrifice",
                    false, // mandatory
                );
            }
            2 => {
                // -6: Separate all permanents target player controls into two piles.
                // That player sacrifices all permanents in the pile of their choice.
                let target_player = match targets.first() {
                    Some(Target::Player(pid)) => *pid,
                    _ => return,
                };

                let permanents: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, target_player)
                    .iter()
                    .map(|o| o.id)
                    .collect();

                if permanents.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -6: p{} has no permanents", target_player.0));
                    return;
                }

                // Liliana's controller divides the permanents into two piles.
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: self_id,
                    choice: ResolutionChoiceKind::DividePermanentsIntoPiles {
                        description: format!(
                            "Liliana -6: divide p{}'s permanents into two piles",
                            target_player.0),
                        permanents,
                        target_player,
                        source_id: self_id,
                    },
                });
            }
            _ => {}
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, _discarded_id: ObjectId, _registry: &CardRegistry) {
        // After a player discards for Liliana +1, chain to the next player.
        self.chain_next_discard(state, self_id);
    }
}

impl LilianaOfTheVeil {
    /// After one player has discarded for the +1, check if more players need to discard
    /// and present the choice to the next one.
    fn chain_next_discard(&self, state: &mut GameState, self_id: ObjectId) {
        // Read remaining player count from card_state.
        let count = state.get_object(self_id)
            .and_then(|o| o.card_state.get("liliana_discard_count").copied())
            .map(|id| id.0 as usize)
            .unwrap_or(0);

        if count == 0 {
            // All players have discarded. Clean up card_state.
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.card_state.remove("liliana_discard_count");
            }
            return;
        }

        // Pop the next player from the remaining list.
        let next_player_id = state.get_object(self_id)
            .and_then(|o| o.card_state.get("liliana_discard_0").copied())
            .map(|id| PlayerId(id.0 as u8))
            .unwrap_or(PlayerId(0));

        // Shift remaining players down and decrement count.
        if let Some(obj) = state.get_object_mut(self_id) {
            for i in 0..(count - 1) {
                let next_key = format!("liliana_discard_{}", i + 1);
                let val = obj.card_state.get(&next_key).copied().unwrap_or(crate::ids::ObjectId(0));
                obj.card_state.insert(format!("liliana_discard_{}", i), val);
            }
            obj.card_state.remove(&format!("liliana_discard_{}", count - 1));
            obj.card_state.insert("liliana_discard_count".into(),
                crate::ids::ObjectId((count - 1) as u64));
        }

        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, next_player_id)
            .iter().map(|o| o.id).collect();

        if hand.is_empty() {
            // This player has no cards — skip and chain to next.
            state.log(crate::state::LogLevel::Event,
                format!("Liliana +1: p{} has no cards to discard", next_player_id.0));
            self.chain_next_discard(state, self_id);
            return;
        }

        if hand.len() == 1 {
            // Only one card — auto-discard.
            let card_id = hand[0];
            let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(card_id, Zone::Graveyard);
            state.events.push(crate::events::GameEvent::Discarded {
                player: next_player_id,
                object: card_id,
            });
            state.log(crate::state::LogLevel::Event,
                format!("Liliana +1: p{} discarded {}", next_player_id.0, name));
            self.chain_next_discard(state, self_id);
            return;
        }

        // Present choice to this player.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: next_player_id,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseCardFromHand {
                description: "Liliana +1: choose a card to discard".into(),
                player: next_player_id,
                cards: hand,
            },
        });
    }
}

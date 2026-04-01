use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Liliana of the Veil {1}{B}{B} Planeswalker — Liliana (3 loyalty).
/// +1: Each player discards a card.
/// -2: Target player sacrifices a creature.
/// -6: Separate all permanents target player controls into two piles.
///     That player sacrifices all permanents in the pile of their choice.
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
            oracle_text: "+1: Each player discards a card.\n-2: Target player sacrifices a creature.\n-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.".into(),
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

    fn loyalty_abilities(&self) -> Vec<LoyaltyAbilityDef> {
        vec![
            LoyaltyAbilityDef {
                ability_index: 0,
                loyalty_change: 1,
                description: "+1: Each player discards a card".into(),
            },
            LoyaltyAbilityDef {
                ability_index: 1,
                loyalty_change: -2,
                description: "-2: Target player sacrifices a creature".into(),
            },
            LoyaltyAbilityDef {
                ability_index: 2,
                loyalty_change: -6,
                description: "-6: Separate permanents into two piles, that player sacrifices one pile".into(),
            },
        ]
    }

    fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        let opponent = state.opponent(controller);

        match ability_index {
            0 => {
                // +1: Each player discards a card.
                // Each player chooses which card to discard. Start with the active player.
                let active = state.active_player;
                let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, active)
                    .iter().map(|o| o.id).collect();
                if hand.len() > 1 {
                    // Active player chooses which card to discard.
                    // Store that we need the other player to discard too.
                    if let Some(obj) = state.get_object_mut(self_id) {
                        obj.card_state.insert("liliana_discard_phase".into(), ObjectId(1));
                    }
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: active,
                        source: self_id,
                        choice: ResolutionChoiceKind::ChooseCardFromHand {
                            description: "Liliana +1: choose a card to discard".into(),
                            player: active,
                            cards: hand,
                            callback_source: Some(self_id),
                        },
                    });
                } else if hand.len() == 1 {
                    // Only one card — auto-discard for active player.
                    let card_id = hand[0];
                    let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
                    state.move_object(card_id, Zone::Graveyard);
                    state.events.push(crate::events::GameEvent::Discarded { player: active, object: card_id });
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana +1: p{} discarded {}", active.0, name));
                    // Now handle the non-active player.
                    liliana_discard_nonactive(state, self_id, active);
                } else {
                    // Active player has no cards. Handle non-active player.
                    liliana_discard_nonactive(state, self_id, active);
                }
            }
            1 => {
                // -2: Target player sacrifices a creature.
                // Present choice: which player to target (in 2-player, opponent is the usual target).
                let mut targets: Vec<Target> = Vec::new();
                for p in &state.players {
                    if !p.lost {
                        // Only include players who have creatures to sacrifice.
                        let has_creatures = state.objects_in_zone(Zone::Battlefield, p.id)
                            .iter().any(|o| o.power.is_some());
                        if has_creatures {
                            targets.push(Target::Player(p.id));
                        }
                    }
                }
                if targets.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        "Liliana -2: no player has creatures to sacrifice".into());
                    return;
                }
                let effect = PendingEffect::ForceSacrificeCreature;
                crate::cards::helpers::present_target_choice(
                    state, self_id, controller, targets, effect,
                    "Liliana -2: choose target player to sacrifice a creature",
                    false,
                );
            }
            2 => {
                // -6: Opponent sacrifices half their permanents (simplified pile division).
                // Per Oracle: "That player sacrifices all permanents in the pile of their choice."
                // The opponent chooses the smaller pile to sacrifice (strategic best).
                let permanents: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, opponent)
                    .iter()
                    .map(|o| o.id)
                    .collect();
                let half = permanents.len() / 2;
                // Split: first half vs second half. Opponent picks smaller pile to sacrifice.
                let to_sacrifice: Vec<ObjectId> = permanents.into_iter().take(half.max(1)).collect();
                for &perm_id in &to_sacrifice {
                    let name = state.get_object(perm_id).map(|o| o.name.clone()).unwrap_or_default();
                    crate::destruction::sacrifice(state, perm_id, registry);
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -6: p{} sacrificed {}", opponent.0, name));
                }
            }
            _ => {}
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, _discarded_id: ObjectId, _registry: &CardRegistry) {
        // After active player discards, handle the non-active player.
        let active = state.active_player;
        let phase = state.get_object(self_id)
            .and_then(|o| o.card_state.get("liliana_discard_phase").copied());
        if phase == Some(ObjectId(1)) {
            // Clear the phase marker.
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.card_state.remove("liliana_discard_phase");
            }
            liliana_discard_nonactive(state, self_id, active);
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        state.add_counters(object_id, CounterType::Loyalty, 3);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.card_types = vec![CardType::Planeswalker];
        }
        state.log(crate::state::LogLevel::Event,
            "Liliana of the Veil enters with 3 loyalty".into());
    }
}

/// Handle the non-active player's discard for Liliana +1.
fn liliana_discard_nonactive(state: &mut GameState, self_id: ObjectId, active: PlayerId) {
    let nonactive = state.opponent(active);
    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, nonactive)
        .iter().map(|o| o.id).collect();
    if hand.len() > 1 {
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: nonactive,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseCardFromHand {
                description: "Liliana +1: choose a card to discard".into(),
                player: nonactive,
                cards: hand,
                callback_source: None,
            },
        });
    } else if hand.len() == 1 {
        let card_id = hand[0];
        let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
        state.move_object(card_id, Zone::Graveyard);
        state.events.push(crate::events::GameEvent::Discarded { player: nonactive, object: card_id });
        state.log(crate::state::LogLevel::Event,
            format!("Liliana +1: p{} discarded {}", nonactive.0, name));
    }
}

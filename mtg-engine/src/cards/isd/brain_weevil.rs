use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Brain Weevil — {3}{B} 1/1 Insect. Intimidate.
/// Sacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.
pub struct BrainWeevil;

impl CardBehavior for BrainWeevil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Brain Weevil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Insect".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)\nSacrifice this creature: Target player discards two cards. Activate only as a sorcery.".into(),
            keywords: vec![Keyword::Intimidate],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Sacrifice: Target player discards two cards".into(),
            cost: ManaCost::new(vec![]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: Some(TargetRequirement::PlayerOnly),
            once_per_turn: false,
            sorcery_speed_only: true,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(target_player)) = targets.first() {
            let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, *target_player)
                .iter()
                .map(|o| o.id)
                .collect();

            if hand.is_empty() {
                return;
            }
            // If exactly 1 card, discard it (no choice needed, still need a second discard of 0).
            if hand.len() == 1 {
                let card_id = hand[0];
                state.move_object(card_id, Zone::Graveyard, registry);
                state.events.push(crate::events::GameEvent::Discarded {
                    player: *target_player,
                    object: card_id,
                });
                state.log(crate::state::LogLevel::Event,
                    format!("Brain Weevil: p{} discarded 1 card (only had 1)", target_player.0));
                return;
            }
            // If exactly 2 cards, discard both (no choice needed).
            if hand.len() == 2 {
                for &card_id in &hand {
                    state.move_object(card_id, Zone::Graveyard, registry);
                    state.events.push(crate::events::GameEvent::Discarded {
                        player: *target_player,
                        object: card_id,
                    });
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Brain Weevil: p{} discarded 2 cards", target_player.0));
                return;
            }
            // 3+ cards: present first choice. Store target player in card_state so
            // on_discard_choice can chain the second discard.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.card_state.insert("weevil_target_player".into(),
                    ObjectId(u64::from(target_player.0)));
            }
            state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                player: *target_player,
                source: object_id,
                choice: crate::state::ResolutionChoiceKind::ChooseCardFromHand {
                    description: "Brain Weevil: choose a card to discard (1 of 2)".into(),
                    player: *target_player,
                    cards: hand,
                },
            });
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, _discarded_id: ObjectId, registry: &CardRegistry) {
        // Retrieve and remove the pending second-discard marker.
        let target_player_raw = state.get_object(self_id)
            .and_then(|o| o.card_state.get("weevil_target_player").copied());
        let Some(raw_id) = target_player_raw else { return };
        let target_player = PlayerId(u8::try_from(raw_id.0).unwrap_or(u8::MAX));

        // Clear the marker so we only chain once.
        if let Some(obj) = state.get_object_mut(self_id) {
            obj.card_state.remove("weevil_target_player");
        }

        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, target_player)
            .iter()
            .map(|o| o.id)
            .collect();

        if hand.is_empty() {
            state.log(crate::state::LogLevel::Event,
                format!("Brain Weevil: p{} has no more cards to discard", target_player.0));
            return;
        }
        if hand.len() == 1 {
            // Only one card left — auto-discard.
            let card_id = hand[0];
            state.move_object(card_id, Zone::Graveyard, registry);
            state.events.push(crate::events::GameEvent::Discarded {
                player: target_player,
                object: card_id,
            });
            state.log(crate::state::LogLevel::Event,
                format!("Brain Weevil: p{} discarded 2nd card", target_player.0));
            return;
        }
        // 2+ cards remaining — present second choice.
        state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
            player: target_player,
            source: self_id,
            choice: crate::state::ResolutionChoiceKind::ChooseCardFromHand {
                description: "Brain Weevil: choose a card to discard (2 of 2)".into(),
                player: target_player,
                cards: hand,
            },
        });
    }
}

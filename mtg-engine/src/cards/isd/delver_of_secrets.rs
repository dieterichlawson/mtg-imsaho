use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Delver of Secrets {U} 1/1 Human Wizard // Insectile Aberration 3/2 Human Insect with Flying.
/// At the beginning of your upkeep, look at the top card of your library. You may reveal that
/// card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
pub struct DelverOfSecrets;

impl DelverOfSecrets {
    /// Check if the top card of the given player's library is an instant or sorcery.
    fn top_card_is_instant_or_sorcery(state: &GameState, controller: crate::ids::PlayerId, registry: &CardRegistry) -> bool {
        let top_card_id = state.get_player(controller).library_order.first().copied();
        if let Some(top_id) = top_card_id {
            let card_id = state.get_object(top_id).map(|o| o.card_id);
            card_id
                .and_then(|cid| registry.card_data(cid))
                .map(|d| d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery))
                .unwrap_or_else(|| {
                    // Fallback to object card_types (for tokens or objects without registry entries).
                    state.get_object(top_id)
                        .map(|o| o.card_types.contains(&CardType::Instant) || o.card_types.contains(&CardType::Sorcery))
                        .unwrap_or(false)
                })
        } else {
            false
        }
    }
}

impl CardBehavior for DelverOfSecrets {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Delver of Secrets".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "look at top card, may reveal to transform".into(),
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Insectile Aberration".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Insect".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "Flying".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((3, 2))
        } else {
            None
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        // Only trigger on the front face, during controller's upkeep.
        if is_transformed || state.active_player != controller {
            return;
        }
        // Look at the top card of the library.
        let top_is_instant_or_sorcery = Self::top_card_is_instant_or_sorcery(state, controller, registry);

        // Log what was seen (the player "looks at" the top card).
        let top_card_name = state.get_player(controller).library_order.first()
            .and_then(|id| state.get_object(*id))
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "nothing".into());
        state.log(LogLevel::Debug,
            format!("Delver of Secrets: top card is {}", top_card_name));

        // Always present the "you may reveal" choice. Per the ruling, the player may reveal
        // any top card. Only if the revealed card is an instant or sorcery does Delver transform.
        let description = if top_is_instant_or_sorcery {
            format!(
                "Delver of Secrets: reveal {} from the top of your library to transform?",
                top_card_name
            )
        } else {
            format!(
                "Delver of Secrets: reveal {} from the top of your library? (not an instant or sorcery — no transform)",
                top_card_name
            )
        };
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description,
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            // Player chose not to reveal — card stays on top, no transform.
            state.log(LogLevel::Event, "Delver of Secrets: chose not to reveal".into());
            return;
        }
        // Player reveals the top card. Only transform if it's an instant or sorcery.
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        let top_card_name = state.get_player(controller).library_order.first()
            .and_then(|id| state.get_object(*id))
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "a card".into());
        let top_is_instant_or_sorcery = Self::top_card_is_instant_or_sorcery(state, controller, registry);
        if top_is_instant_or_sorcery {
            state.log(LogLevel::Event,
                format!("Delver of Secrets: reveals {} — transforming!", top_card_name));
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = true;
                obj.name = "Insectile Aberration".into();
            }
        } else {
            state.log(LogLevel::Event,
                format!("Delver of Secrets: reveals {} — not an instant or sorcery, no transform.", top_card_name));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

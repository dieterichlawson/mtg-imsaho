use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind, YesNoEffect};
use crate::types::*;

/// Delver of Secrets {U} 1/1 Human Wizard // Insectile Aberration 3/2 Human Insect with Flying.
/// At the beginning of your upkeep, look at the top card of your library. You may reveal that
/// card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
pub struct DelverOfSecrets;

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
            oracle_text: "At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "look at top card, may transform".into(),
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
        // Look at the top card of the library. If it's an instant or sorcery, transform.
        let top_card_id = state.get_player(controller).library_order.first().copied();
        if let Some(top_id) = top_card_id {
            // Check card types via registry (more reliable) or object.
            let card_id = state.get_object(top_id).map(|o| o.card_id);
            let is_instant_or_sorcery = card_id
                .and_then(|cid| registry.card_data(cid))
                .map(|d| d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery))
                .unwrap_or_else(|| {
                    // Fallback to object card_types.
                    state.get_object(top_id)
                        .map(|o| o.card_types.contains(&CardType::Instant) || o.card_types.contains(&CardType::Sorcery))
                        .unwrap_or(false)
                });
            if is_instant_or_sorcery {
                // "You may reveal that card. If an instant or sorcery card is
                // revealed this way, transform Delver of Secrets."
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: self_id,
                    choice: ResolutionChoiceKind::YesNo {
                        description: "Reveal top card and transform Delver of Secrets?".into(),
                        source_card: self_id,
                        effect: YesNoEffect::Transform { back_face_name: "Insectile Aberration".into() },
                    },
                });
            }
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

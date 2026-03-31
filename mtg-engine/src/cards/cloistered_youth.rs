use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Cloistered Youth {1}{W} 3/2 Human // Unholy Fiend 3/3 Horror.
/// At the beginning of your upkeep, you may transform Cloistered Youth.
/// Unholy Fiend: At the beginning of your upkeep, you lose 1 life.
///
/// Simplified: Automatically transforms at your upkeep (front→back). Back face deals 1 damage
/// to you each upkeep and doesn't transform back (per the original card, back face has no
/// transform-back trigger — it stays as Unholy Fiend).
pub struct CloisteredYouth;

impl CardBehavior for CloisteredYouth {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Cloistered Youth".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "At the beginning of your upkeep, you may transform Cloistered Youth.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "may transform / lose 1 life".into(),
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Unholy Fiend".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Horror".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "At the beginning of your upkeep, you lose 1 life.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((3, 3))
        } else {
            None
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        if !is_transformed {
            // Transform from Cloistered Youth to Unholy Fiend.
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = true;
                obj.name = "Unholy Fiend".into();
            }
            state.log(crate::state::LogLevel::Event,
                "Cloistered Youth transforms into Unholy Fiend".into());
        } else {
            // Unholy Fiend: lose 1 life.
            state.get_player_mut(controller).life -= 1;
            state.log(crate::state::LogLevel::Event,
                format!("Unholy Fiend: p{} loses 1 life", controller.0));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

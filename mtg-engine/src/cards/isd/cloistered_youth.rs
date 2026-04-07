use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::cards::helpers;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Cloistered Youth {1}{W} 1/1 Human // Unholy Fiend 3/3 Horror.
/// Front: At the beginning of your upkeep, you may transform Cloistered Youth.
/// Back: At the beginning of your end step, you lose 1 life.
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
            power: Some(1),
            toughness: Some(1),
            oracle_text: "At the beginning of your upkeep, you may transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "you may transform Cloistered Youth".into(),
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
            oracle_text: "At the beginning of your end step, you lose 1 life.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "lose 1 life".into(),
                },
            ],
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
            // "You may transform Cloistered Youth" — present choice to the player.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::YesNo {
                    description: "Cloistered Youth: transform into Unholy Fiend?".into(),
                    source_card: self_id,
                },
            });
        }
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            state.log(LogLevel::Event,
                "Cloistered Youth: chose not to transform".into());
            return;
        }

        // Transform using the generic helper (updates name, keywords, subtypes, is_transformed).
        helpers::apply_transform(state, self_id, registry);
        let new_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
        state.log(LogLevel::Event,
            format!("Cloistered Youth transforms into {}", new_name));
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        if is_transformed {
            // Unholy Fiend: lose 1 life at end step.
            let old = state.get_player(controller).life;
            let new_life = old - 1;
            state.get_player_mut(controller).life = new_life;
            state.events.push(crate::events::GameEvent::LifeChanged { player: controller, old, new_life });
            state.log(LogLevel::Event,
                format!("Unholy Fiend: p{} loses 1 life", controller.0));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

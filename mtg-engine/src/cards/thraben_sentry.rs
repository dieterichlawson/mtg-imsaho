use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind, YesNoEffect};
use crate::types::*;

/// Thraben Sentry {3}{W} 2/2 Human Soldier with Vigilance // Thraben Militia 5/4 Human Soldier.
/// Whenever another creature you control dies, you may transform Thraben Sentry.
pub struct ThrabenSentry;

impl CardBehavior for ThrabenSentry {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Thraben Sentry".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Vigilance\nWhenever another creature you control dies, you may transform Thraben Sentry.".into(),
            keywords: vec![Keyword::Vigilance],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "may transform Thraben Sentry".into(),
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Thraben Militia".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(5),
            toughness: Some(4),
            oracle_text: "Trample".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((5, 4))
        } else {
            None
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        // Only trigger when another creature WE control dies, and only on front face.
        if dead_controller != controller || is_transformed {
            return;
        }
        // "You may transform Thraben Sentry" — present choice.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Transform Thraben Sentry into Thraben Militia?".into(),
                source_card: self_id,
                effect: YesNoEffect::Transform { back_face_name: "Thraben Militia".into() },
            },
        });
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

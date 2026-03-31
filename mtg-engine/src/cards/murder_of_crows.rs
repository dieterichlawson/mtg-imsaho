use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::*;

/// Murder of Crows — {3}{U}{U} 4/4 Bird. Flying.
/// Whenever another creature dies, you may draw a card. If you do, discard a card.
pub struct MurderOfCrows;

impl CardBehavior for MurderOfCrows {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Murder of Crows".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Bird".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "you may draw a card, then discard a card".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // "You may draw a card. If you do, discard a card."
        // Present yes/no choice. If yes, the engine draws and presents discard.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Murder of Crows: draw a card? (you must then discard one)".into(),
                source_card: self_id,
            },
        });
    }
}

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
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "you may draw a card, then discard a card".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // "You may draw a card. If you do, discard a card."
        // Draw the card (the "may" makes this optional, but in almost all cases
        // you want to draw — and the player can always choose to discard the
        // drawn card to break even). Auto-draw, then present discard choice.
        crate::engine::draw_cards(state, controller, 1);

        // Present discard choice from hand.
        let hand: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Hand && o.owner == controller)
            .map(|o| o.id)
            .collect();

        if hand.is_empty() {
            return;
        }

        if hand.len() == 1 {
            // Only one card in hand — auto-discard it.
            let card = hand[0];
            let name = state.get_object(card).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(card, Zone::Graveyard);
            state.log(crate::state::LogLevel::Event,
                format!("Murder of Crows: p{} drew and discarded {}", controller.0, name));
        } else {
            // Multiple cards — let the player choose which to discard.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseCardFromHand {
                    description: "Murder of Crows: choose a card to discard".into(),
                    player: controller,
                    cards: hand,
                },
            });
            state.log(crate::state::LogLevel::Event,
                format!("Murder of Crows: p{} drew a card, must discard one", controller.0));
        }
    }
}

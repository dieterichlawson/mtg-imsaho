use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Forbidden Alchemy — {2}{U} instant. Look at the top four cards of your library.
/// Put one into your hand and the rest into your graveyard.
/// Simplified: draw 1 card, mill 3.
pub struct ForbiddenAlchemy;

impl CardBehavior for ForbiddenAlchemy {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Forbidden Alchemy".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.\nFlashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(6), ManaSymbol::Colored(Color::Black)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        // Remove top 4 cards from library_order (they stay in Zone::Library but aren't drawable).
        let player = state.get_player_mut(controller);
        let count = std::cmp::min(4, player.library_order.len());
        let revealed: Vec<ObjectId> = player.library_order.drain(..count).collect();

        if revealed.is_empty() {
            // No cards to reveal.
            state.move_spell_after_resolve(object_id);
        } else if revealed.len() == 1 {
            // Only 1 card -- auto-put it in hand.
            let card_id = revealed[0];
            state.move_object(card_id, Zone::Hand);
            let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(LogLevel::Event, format!("Forbidden Alchemy: {} put into hand", name));
            state.move_spell_after_resolve(object_id);
        } else {
            // 2+ cards -- ask the player which one to keep.
            let names: Vec<String> = revealed.iter()
                .filter_map(|id| state.get_object(*id).map(|o| o.name.clone()))
                .collect();
            state.log(LogLevel::Event, format!("Forbidden Alchemy revealed: {}", names.join(", ")));
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseFromRevealed {
                    description: "Forbidden Alchemy: choose a card to put into your hand (rest go to graveyard)".into(),
                    revealed,
                    spell_id: object_id,
                },
            });
            // Don't clean up spell yet -- ResolveChoice handler does it.
        }
    }
}

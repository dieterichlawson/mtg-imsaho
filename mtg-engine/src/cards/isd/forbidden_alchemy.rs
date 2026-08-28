use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Forbidden Alchemy — {2}{U} instant. Look at the top four cards of your library.
/// Put one into your hand and the rest into your graveyard.
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
            oracle_text: "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.\nFlashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(6), ManaSymbol::Colored(Color::Black)])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Look at the top four cards of your library" — looking moves
        // nothing (CR 701.16a), so they stay in the library and stay in its
        // order until the choice is answered. This used to drain them out of
        // `library_order` while leaving their zone as `Library`, which left
        // the library's two halves disagreeing for as long as the prompt was
        // open. Every one of them leaves in the answer, and `move_object`
        // takes each out of the order then.
        let library = &state.get_player(controller).library_order;
        let revealed: Vec<ObjectId> = library.iter().take(4).copied().collect();

        if revealed.is_empty() {
            // No cards to reveal.
        } else if revealed.len() == 1 {
            // Only 1 card -- auto-put it in hand.
            let card_id = revealed[0];
            let chosen_name = state.obj_name(card_id);
            state.move_object(card_id, Zone::Hand, registry);
            state.log(LogLevel::Event, format!("Forbidden Alchemy: {chosen_name} put into hand"));
        } else {
            // 2+ cards -- ask the player which one to keep.
            let names: Vec<String> = revealed.iter()
                .map(|id| state.obj_name(*id))
                .collect();
            state.log(LogLevel::Event, format!("Forbidden Alchemy revealed: {}", names.join(", ")));
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseFromRevealed {
                    description: "Forbidden Alchemy: choose a card to put into your hand (rest go to graveyard)".into(),
                    revealed,
                },
            });
            // Don't clean up spell yet -- ResolveChoice handler does it.
        }
    }
}

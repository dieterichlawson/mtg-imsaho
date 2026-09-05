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
        let looked_at: Vec<ObjectId> = library.iter().take(4).copied().collect();

        if looked_at.is_empty() {
            // Nothing to look at.
        } else if looked_at.len() == 1 {
            // Only 1 card -- auto-put it in hand.
            let card_id = looked_at[0];
            let chosen_name = state.obj_name(card_id);
            state.move_object(card_id, Zone::Hand, registry);
            state.log(LogLevel::Private, format!("Forbidden Alchemy: {chosen_name} put into hand"));
            state.log(LogLevel::Event, "Forbidden Alchemy: a card put into hand".to_string());
        } else {
            // 2+ cards -- ask the player which one to keep. Which cards they
            // are is the caster's alone: "look at" is not "reveal" (CR
            // 701.18a), so the names go in at Private, which neither the
            // shared --log nor either seat's LOG pane carries (issue #217).
            let names: Vec<String> = looked_at.iter()
                .map(|id| state.obj_name(*id))
                .collect();
            state.log(LogLevel::Private, format!("Forbidden Alchemy looked at: {}", names.join(", ")));
            state.log(LogLevel::Event,
                format!("Forbidden Alchemy: p{} looks at the top {} cards of their library",
                    controller.0, names.len()));
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseFromLookedAt {
                    description: "Forbidden Alchemy: choose a card to put into your hand (rest go to graveyard)".into(),
                    looked_at,
                },
            });
            // Don't clean up spell yet -- ResolveChoice handler does it.
        }
    }
}

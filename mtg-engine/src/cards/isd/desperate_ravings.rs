use rand::seq::SliceRandom;

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Desperate Ravings — {1}{R} instant. Draw two cards, then discard a card at random.
pub struct DesperateRavings;

impl CardBehavior for DesperateRavings {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Desperate Ravings".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Draw two cards, then discard a card at random.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);
        let _ = crate::engine::draw_cards(state, controller, 2, registry);
        // Discard a card at random.
        let hand: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Hand && o.owner == controller)
            .map(|o| o.id)
            .collect();
        let to_discard = hand.choose(&mut rand::thread_rng()).copied();
        if let Some(discard_id) = to_discard {
            state.discard_card(discard_id, registry);
        }
    }
}

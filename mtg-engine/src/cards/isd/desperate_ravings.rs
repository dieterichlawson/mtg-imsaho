use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
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
        let controller = crate::cards::helpers::controller_of(state, object_id);
        let _ = crate::engine::draw_cards(state, controller, 2, registry);
        // Discard a card at random.
        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, controller).into_iter()
            .map(|o| o.id)
            .collect();
        // "then discard a card AT RANDOM".
        let to_discard = state.choose_at_random(&hand, 1).first().copied();
        if let Some(discard_id) = to_discard {
            state.discard_card(discard_id, registry);
        }
    }
}

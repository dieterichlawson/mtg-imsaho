use rand::seq::SliceRandom;

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Draw two cards, then discard a card at random.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        crate::engine::draw_cards(state, controller, 2);
        // Discard a card at random.
        let hand: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Hand && o.owner == controller)
            .map(|o| o.id)
            .collect();
        let to_discard = hand.choose(&mut rand::thread_rng()).copied();
        if let Some(discard_id) = to_discard {
            let owner = state.get_object(discard_id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
            state.move_object(discard_id, Zone::Graveyard);
            state.events.push(crate::events::GameEvent::Discarded { player: owner, object: discard_id });
        }
        state.move_spell_after_resolve(object_id);
    }
}

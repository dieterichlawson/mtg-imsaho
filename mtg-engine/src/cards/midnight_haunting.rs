use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Midnight Haunting — {2}{W} instant. Create two 1/1 white Spirit tokens with flying.
pub struct MidnightHaunting;

impl CardBehavior for MidnightHaunting {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Midnight Haunting".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Create two 1/1 white Spirit creature tokens with flying.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        for _ in 0..2 {
            state.create_token("Spirit", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying]);
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

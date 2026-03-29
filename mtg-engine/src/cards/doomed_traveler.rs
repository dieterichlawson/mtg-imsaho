use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Doomed Traveler — {W} 1/1 Human Soldier. When it dies, create a 1/1 white Spirit token with flying.
pub struct DoomedTraveler;

impl CardBehavior for DoomedTraveler {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Doomed Traveler".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "When Doomed Traveler dies, create a 1/1 white Spirit creature token with flying.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let owner = state.get_object(object_id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
        state.create_token("Spirit", owner, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying]);
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Mausoleum Guard — {3}{W} 2/2 Human Scout. When it dies, create two 1/1 white Spirit tokens with flying.
pub struct MausoleumGuard;

impl CardBehavior for MausoleumGuard {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mausoleum Guard".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Scout".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When Mausoleum Guard dies, create two 1/1 white Spirit creature tokens with flying.".into(),
            keywords: vec![],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let owner = state.get_object(object_id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
        for _ in 0..2 {
            state.create_token("Spirit", owner, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying]);
        }
    }
}

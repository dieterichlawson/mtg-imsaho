use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Fortress Crab — 1/6 for {3}{U}. Vanilla creature.
pub struct FortressCrab;

impl CardBehavior for FortressCrab {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Fortress Crab".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Crab".into()],
            power: Some(1),
            toughness: Some(6),
            oracle_text: String::new(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![],
        }
    }
}

use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Coral Merfolk — 2/1 for {1}{U}. Vanilla creature.
pub struct CoralMerfolk;

impl CardBehavior for CoralMerfolk {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Coral Merfolk".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Merfolk".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: String::new(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![],
        }
    }
}

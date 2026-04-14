use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Riot Devils — 2/3 for {2}{R}. Vanilla creature.
pub struct RiotDevils;

impl CardBehavior for RiotDevils {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Riot Devils".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Devil".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: String::new(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

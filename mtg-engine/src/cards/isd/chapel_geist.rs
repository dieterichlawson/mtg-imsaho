use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Chapel Geist — 2/3 for {1}{W}{W}. Flying.
pub struct ChapelGeist;

impl CardBehavior for ChapelGeist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Chapel Geist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Flying".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

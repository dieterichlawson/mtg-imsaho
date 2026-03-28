use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Somberwald Spider — 2/4 for {4}{G}. Reach.
pub struct SomberwaldSpider;

impl CardBehavior for SomberwaldSpider {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Somberwald Spider".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spider".into()],
            power: Some(2),
            toughness: Some(4),
            oracle_text: "Reach".into(),
            keywords: vec![Keyword::Reach],
            flashback_cost: None,
        }
    }
}

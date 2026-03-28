use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Savannah Lions — 2/1 for {W}. Vanilla creature.
pub struct SavannahLions;

impl CardBehavior for SavannahLions {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Savannah Lions".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Cat".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: String::new(),
            keywords: vec![],
            flashback_cost: None,
        }
    }
}

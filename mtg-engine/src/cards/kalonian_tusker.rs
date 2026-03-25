use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Kalonian Tusker — 3/3 for {G}{G}. Vanilla creature.
pub struct KalonianTusker;

impl CardBehavior for KalonianTusker {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kalonian Tusker".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Beast".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: String::new(),
        }
    }
}

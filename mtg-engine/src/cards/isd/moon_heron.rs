use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Moon Heron — 3/2 for {3}{U}. Flying.
pub struct MoonHeron;

impl CardBehavior for MoonHeron {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moon Heron".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into(), "Bird".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "Flying".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        }
    }
}

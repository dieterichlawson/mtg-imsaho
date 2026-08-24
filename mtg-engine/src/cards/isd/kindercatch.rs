use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Kindercatch — 6/6 for {3}{G}{G}{G}. Vanilla creature.
pub struct Kindercatch;

impl CardBehavior for Kindercatch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kindercatch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into()],
            power: Some(6),
            toughness: Some(6),
            ..Default::default()
        }
    }
}

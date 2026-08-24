use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Thraben Purebloods — 3/5 for {4}{W}. Vanilla creature.
pub struct ThrabenPurebloods;

impl CardBehavior for ThrabenPurebloods {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Thraben Purebloods".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Dog".into()],
            power: Some(3),
            toughness: Some(5),
            ..Default::default()
        }
    }
}

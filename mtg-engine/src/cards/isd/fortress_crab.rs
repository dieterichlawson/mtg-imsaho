use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
            subtypes: vec!["Crab".into()],
            power: Some(1),
            toughness: Some(6),
            ..Default::default()
        }
    }
}

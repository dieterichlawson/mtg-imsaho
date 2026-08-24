use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Goblin Piker — 2/1 for {1}{R}. Vanilla creature.
pub struct GoblinPiker;

impl CardBehavior for GoblinPiker {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Goblin Piker".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Goblin".into(), "Warrior".into()],
            power: Some(2),
            toughness: Some(1),
            ..Default::default()
        }
    }
}

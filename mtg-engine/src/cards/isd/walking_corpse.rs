use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Walking Corpse — 2/2 for {1}{B}. Vanilla creature.
pub struct WalkingCorpse;

impl CardBehavior for WalkingCorpse {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Walking Corpse".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            ..Default::default()
        }
    }
}

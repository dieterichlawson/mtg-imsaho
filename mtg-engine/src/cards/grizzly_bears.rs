use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Grizzly Bears — 2/2 for {1}{G}. Vanilla creature.
pub struct GrizzlyBears;

impl CardBehavior for GrizzlyBears {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grizzly Bears".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Bear".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: String::new(),
        }
    }
}

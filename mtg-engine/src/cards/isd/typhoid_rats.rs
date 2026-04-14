use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Typhoid Rats — 1/1 for {B}. Deathtouch.
pub struct TyphoidRats;

impl CardBehavior for TyphoidRats {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Typhoid Rats".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Rat".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.)".into(),
            keywords: vec![Keyword::Deathtouch],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

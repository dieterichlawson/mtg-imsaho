use crate::cards::{CardBehavior, CardData};
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: String::new(),
            keywords: vec![],
            flashback_cost: None,
        }
    }
}

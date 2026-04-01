use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Rotting Fensnake — 5/1 for {3}{B}. Vanilla creature.
pub struct RottingFensnake;

impl CardBehavior for RottingFensnake {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rotting Fensnake".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Snake".into()],
            power: Some(5),
            toughness: Some(1),
            oracle_text: String::new(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

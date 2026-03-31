use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Laboratory Maniac — {2}{U} 2/2 Human Wizard.
/// If you would draw a card while your library has no cards in it, you win the game instead.
pub struct LaboratoryManiac;

impl CardBehavior for LaboratoryManiac {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Laboratory Maniac".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "If you would draw a card while your library has no cards in it, you win the game instead.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

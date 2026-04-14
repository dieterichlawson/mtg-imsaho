use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Abbey Griffin — 2/2 for {3}{W}. Flying, vigilance.
pub struct AbbeyGriffin;

impl CardBehavior for AbbeyGriffin {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Abbey Griffin".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Griffin".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying, vigilance".into(),
            keywords: vec![Keyword::Flying, Keyword::Vigilance],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

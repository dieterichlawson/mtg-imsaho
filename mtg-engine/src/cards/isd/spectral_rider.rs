use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Spectral Rider — 2/2 for {W}{W}. Intimidate.
pub struct SpectralRider;

impl CardBehavior for SpectralRider {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spectral Rider".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into(), "Knight".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)".into(),
            keywords: vec![Keyword::Intimidate],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

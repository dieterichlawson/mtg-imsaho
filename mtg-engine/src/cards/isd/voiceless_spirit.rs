use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Voiceless Spirit — 2/1 for {2}{W}. Flying.
pub struct VoicelessSpirit;

impl CardBehavior for VoicelessSpirit {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Voiceless Spirit".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flying, first strike".into(),
            keywords: vec![Keyword::Flying, Keyword::FirstStrike],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

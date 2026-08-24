use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Ambush Viper — 2/1 for {1}{G}. Flash, deathtouch.
pub struct AmbushViper;

impl CardBehavior for AmbushViper {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ambush Viper".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Snake".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flash\nDeathtouch".into(),
            keywords: vec![Keyword::Flash, Keyword::Deathtouch],
            ..Default::default()
        }
    }
}

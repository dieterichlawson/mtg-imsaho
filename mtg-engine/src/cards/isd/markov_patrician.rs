use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Markov Patrician — 3/1 for {2}{B}. Lifelink.
pub struct MarkovPatrician;

impl CardBehavior for MarkovPatrician {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Markov Patrician".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(1),
            oracle_text: "Lifelink (Damage dealt by this creature also causes you to gain that much life.)".into(),
            keywords: vec![Keyword::Lifelink],
            ..Default::default()
        }
    }
}

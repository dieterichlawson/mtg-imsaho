use crate::cards::{AdditionalCost, CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Stitched Drake — {1}{U}{U} 3/4 Zombie Drake with Flying.
/// As an additional cost to cast this spell, exile a creature card from your graveyard. Flying.
pub struct StitchedDrake;

impl CardBehavior for StitchedDrake {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stitched Drake".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into(), "Drake".into()],
            power: Some(3),
            toughness: Some(4),
            oracle_text: "As an additional cost to cast this spell, exile a creature card from your graveyard.\nFlying".into(),
            keywords: vec![Keyword::Flying],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            ..Default::default()
        }
    }

}

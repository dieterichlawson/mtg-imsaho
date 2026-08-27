use crate::cards::{AdditionalCost, CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Makeshift Mauler — {3}{U} 4/5 Zombie.
/// As an additional cost to cast this spell, exile a creature card from your graveyard.
pub struct MakeshiftMauler;

impl CardBehavior for MakeshiftMauler {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Makeshift Mauler".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into(), "Horror".into()],
            power: Some(4),
            toughness: Some(5),
            oracle_text: "As an additional cost to cast this spell, exile a creature card from your graveyard.".into(),
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            ..Default::default()
        }
    }

}

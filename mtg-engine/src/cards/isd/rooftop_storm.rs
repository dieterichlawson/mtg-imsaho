use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Rooftop Storm — {5}{U} Enchantment.
/// You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
pub struct RooftopStorm;

impl CardBehavior for RooftopStorm {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rooftop Storm".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::AlternativeCost {
                    cost: ManaCost::free(),
                    filter: SpellFilter::CreatureWithSubtype("Zombie".into()),
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }
}

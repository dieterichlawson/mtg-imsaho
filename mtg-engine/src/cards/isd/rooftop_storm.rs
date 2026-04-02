use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Rooftop Storm — {5}{U} Enchantment.
/// You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
///
/// Implementation: The engine checks for Rooftop Storm via `rooftop_storm_applies()`
/// in `engine.rs` and generates alternative-cost CastSpell actions with `ManaCost::free()`
/// when the controller casts a Zombie creature spell.
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
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }
}

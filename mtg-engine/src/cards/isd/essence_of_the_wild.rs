use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ReplacementEffect};

/// Essence of the Wild {3}{G}{G}{G} 6/6 Avatar.
/// Creatures you control enter as a copy of Essence of the Wild.
///
/// This is a replacement effect (CR 614.1d): the creature never exists in its
/// original form on the battlefield. The engine checks for `EnterAsCopy` via
/// the card registry in `apply_entering_copy_replacement`.
pub struct EssenceOfTheWild;

impl CardBehavior for EssenceOfTheWild {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Essence of the Wild".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Avatar".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Creatures you control enter as a copy of this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn replacement_effects(&self) -> Vec<ReplacementEffect> {
        vec![ReplacementEffect::EnterAsCopy]
    }
}

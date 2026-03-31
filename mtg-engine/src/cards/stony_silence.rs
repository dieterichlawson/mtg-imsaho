use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Stony Silence — {1}{W} Enchantment.
/// Activated abilities of artifacts can't be activated.
///
/// Known limitation: the engine doesn't have an ability restriction system.
/// This card is registered for deck building and oracle text purposes, but its
/// static ability is not enforced. Implementing it would require the engine to
/// check for Stony Silence when generating legal activated ability actions for
/// artifacts.
pub struct StonySilence;

impl CardBehavior for StonySilence {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stony Silence".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Activated abilities of artifacts can't be activated.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }
}

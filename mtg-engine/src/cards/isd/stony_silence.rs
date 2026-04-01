use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Stony Silence — {1}{W} Enchantment.
/// Activated abilities of artifacts can't be activated.
///
/// Enforced by the engine in legal_actions(): when Stony Silence is on the
/// battlefield, both mana abilities and non-mana activated abilities of
/// artifacts are excluded from the legal action list.
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

use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Stony Silence — {1}{W} Enchantment.
/// Activated abilities of artifacts can't be activated.
///
/// Declares PreventArtifactAbilities as a continuous effect. The engine
/// checks for this effect in legal_actions() to exclude artifact abilities.
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
            flashback_cost: None,
            continuous_effects: vec![ContinuousEffect::PreventArtifactAbilities],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }
}

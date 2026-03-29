use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Intangible Virtue — {1}{W} enchantment. Creature tokens you control get +1/+1 and have vigilance.
pub struct IntangibleVirtue;

impl CardBehavior for IntangibleVirtue {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Intangible Virtue".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Creature tokens you control get +1/+1 and have vigilance.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 1, scope: EffectScope::Global(CreatureFilter::YourTokens) },
                ContinuousEffect::GrantKeyword { keyword: Keyword::Vigilance, scope: EffectScope::Global(CreatureFilter::YourTokens) },
            ],
        }
    }
}

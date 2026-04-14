use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope, CreatureFilter};

/// Glorious Anthem — {1}{W}{W} enchantment. Creatures you control get +1/+1.
/// The bonus is computed dynamically by `GameState::anthem_power_bonus/anthem_toughness_bonus`.
pub struct GloriousAnthem;

impl CardBehavior for GloriousAnthem {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Glorious Anthem".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Creatures you control get +1/+1.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 1, scope: EffectScope::Global(CreatureFilter::ControlledByYou) },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }
}

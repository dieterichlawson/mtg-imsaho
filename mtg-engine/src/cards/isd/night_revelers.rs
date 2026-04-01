use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Night Revelers — {4}{R} 4/4 Vampire.
/// Night Revelers has haste as long as an opponent controls a Human.
pub struct NightRevelers;

impl CardBehavior for NightRevelers {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Night Revelers".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Night Revelers has haste as long as an opponent controls a Human.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ConditionalKeyword {
                    keyword: Keyword::Haste,
                    condition: EffectCondition::OpponentControlsSubtype("Human".into()),
                    scope: EffectScope::OnSelf,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }
}

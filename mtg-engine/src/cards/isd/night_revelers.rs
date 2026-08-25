use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, Keyword, EffectCondition, EffectScope};

/// Night Revelers — {4}{R} 4/4 Vampire.
/// This creature has haste as long as an opponent controls a Human.
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
            subtypes: vec!["Vampire".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "This creature has haste as long as an opponent controls a Human.".into(),
            continuous_effects: vec![
                ContinuousEffect::when(
                    EffectCondition::OpponentControlsSubtype("Human".into()),
                    ContinuousEffect::GrantKeyword { keyword: Keyword::Haste, scope: EffectScope::OnSelf },
                ),
            ],
            ..Default::default()
        }
    }
}

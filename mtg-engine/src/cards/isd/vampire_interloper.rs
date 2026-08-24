use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope};

/// Vampire Interloper — 2/1 for {1}{B}. Flying, can't block.
pub struct VampireInterloper;

impl CardBehavior for VampireInterloper {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Vampire Interloper".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into(), "Scout".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flying\nThis creature can't block.".into(),
            keywords: vec![Keyword::Flying],
            continuous_effects: vec![
                ContinuousEffect::PreventBlock { scope: EffectScope::OnSelf },
            ],
            ..Default::default()
        }
    }
}

use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectCondition, EffectScope};

/// Angelic Overseer — {3}{W}{W} 5/3 Angel.
/// Flying.
/// As long as you control a Human, this creature has hexproof and indestructible.
pub struct AngelicOverseer;

impl CardBehavior for AngelicOverseer {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Angelic Overseer".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Angel".into()],
            power: Some(5),
            toughness: Some(3),
            oracle_text: "Flying\nAs long as you control a Human, this creature has hexproof and indestructible.".into(),
            keywords: vec![Keyword::Flying],
            continuous_effects: vec![
                ContinuousEffect::when(
                    EffectCondition::YouControlSubtype("Human".into()),
                    ContinuousEffect::GrantKeyword { keyword: Keyword::Hexproof, scope: EffectScope::OnSelf },
                ),
                ContinuousEffect::when(
                    EffectCondition::YouControlSubtype("Human".into()),
                    ContinuousEffect::GrantKeyword { keyword: Keyword::Indestructible, scope: EffectScope::OnSelf },
                ),
            ],
            ..Default::default()
        }
    }
}

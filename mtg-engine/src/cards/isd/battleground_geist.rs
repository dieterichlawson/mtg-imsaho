use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope, CreatureFilter};

/// Battleground Geist — {4}{U} 3/3 Spirit, Flying.
/// Other Spirit creatures you control get +1/+0.
pub struct BattlegroundGeist;

impl CardBehavior for BattlegroundGeist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Battleground Geist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Flying\nOther Spirit creatures you control get +1/+0.".into(),
            keywords: vec![Keyword::Flying],
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 1,
                    toughness: 0,
                    scope: EffectScope::GlobalOther(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::HasSubtype("Spirit".into()),
                    ])),
                },
            ],
            ..Default::default()
        }
    }
}

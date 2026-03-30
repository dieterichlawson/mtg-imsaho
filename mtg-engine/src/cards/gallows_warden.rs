use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Gallows Warden — {4}{W} 3/3 Spirit, Flying.
/// Other Spirit creatures you control get +0/+1.
pub struct GallowsWarden;

impl CardBehavior for GallowsWarden {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Gallows Warden".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Flying\nOther Spirit creatures you control get +0/+1.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 0,
                    toughness: 1,
                    scope: EffectScope::GlobalOther(CreatureFilter::And(vec![
                        CreatureFilter::You,
                        CreatureFilter::HasSubtype("Spirit".into()),
                    ])),
                },
            ],
            triggered_abilities: vec![],
        }
    }
}

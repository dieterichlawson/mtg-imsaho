use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Orchard Spirit — {2}{G} 2/2 Spirit.
/// Orchard Spirit can't be blocked except by creatures with flying or reach.
pub struct OrchardSpirit;

impl CardBehavior for OrchardSpirit {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Orchard Spirit".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Orchard Spirit can't be blocked except by creatures with flying or reach.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::BlockRestriction {
                    allowed_blockers: CreatureFilter::Or(vec![
                        CreatureFilter::HasKeyword(Keyword::Flying),
                        CreatureFilter::HasKeyword(Keyword::Reach),
                    ]),
                    scope: EffectScope::OnSelf,
                },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }
}

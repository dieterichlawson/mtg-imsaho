use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// One-Eyed Scarecrow — 2/3 for {3}. Artifact creature with defender.
/// Creatures with flying your opponents control get -1/-0.
pub struct OneEyedScarecrow;

impl CardBehavior for OneEyedScarecrow {
    fn card_data(&self) -> CardData {
        CardData {
            name: "One-Eyed Scarecrow".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
            ])),
            card_types: vec![CardType::Artifact, CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Scarecrow".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Defender. Creatures with flying your opponents control get -1/-0.".into(),
            keywords: vec![Keyword::Defender],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: -1, toughness: 0, scope: EffectScope::Global(CreatureFilter::And(vec![CreatureFilter::Opponents, CreatureFilter::HasKeyword(Keyword::Flying)])) },
            ],
        }
    }
}

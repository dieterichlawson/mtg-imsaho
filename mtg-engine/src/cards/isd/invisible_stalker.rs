use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope};

/// Invisible Stalker — 1/1 for {1}{U}. Hexproof, can't be blocked.
pub struct InvisibleStalker;

impl CardBehavior for InvisibleStalker {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Invisible Stalker".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Rogue".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Hexproof (This creature can't be the target of spells or abilities your opponents control.)\nThis creature can't be blocked.".into(),
            keywords: vec![Keyword::Hexproof],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::CantBeBlocked { scope: EffectScope::OnSelf },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }
}

use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Grave Bramble — 3/4 for {1}{G}{G}. Defender, protection from Zombies.
pub struct GraveBramble;

impl CardBehavior for GraveBramble {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grave Bramble".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Plant".into()],
            power: Some(3),
            toughness: Some(4),
            oracle_text: "Defender, protection from Zombies".into(),
            keywords: vec![Keyword::Defender],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ProtectionFromSubtype { subtype: "Zombie".into(), scope: EffectScope::OnSelf },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }
}

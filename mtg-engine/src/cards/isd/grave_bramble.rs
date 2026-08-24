use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope};

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
            subtypes: vec!["Plant".into()],
            power: Some(3),
            toughness: Some(4),
            oracle_text: "Defender, protection from Zombies".into(),
            keywords: vec![Keyword::Defender],
            continuous_effects: vec![
                ContinuousEffect::ProtectionFromSubtype { subtype: "Zombie".into(), scope: EffectScope::OnSelf },
            ],
            ..Default::default()
        }
    }
}

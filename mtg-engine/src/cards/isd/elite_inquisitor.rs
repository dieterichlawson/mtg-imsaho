use crate::cards::{CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope};

/// Elite Inquisitor — {W}{W} 2/2 Human Soldier.
/// First strike, vigilance.
/// Protection from Vampires, from Werewolves, and from Zombies.
pub struct EliteInquisitor;

impl CardBehavior for EliteInquisitor {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Elite Inquisitor".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "First strike, vigilance\nProtection from Vampires, from Werewolves, and from Zombies".into(),
            keywords: vec![Keyword::FirstStrike, Keyword::Vigilance],
            continuous_effects: vec![
                ContinuousEffect::ProtectionFromSubtype { subtype: "Vampire".into(), scope: EffectScope::OnSelf },
                ContinuousEffect::ProtectionFromSubtype { subtype: "Werewolf".into(), scope: EffectScope::OnSelf },
                ContinuousEffect::ProtectionFromSubtype { subtype: "Zombie".into(), scope: EffectScope::OnSelf },
            ],
            ..Default::default()
        }
    }
}

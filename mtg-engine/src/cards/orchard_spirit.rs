use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Orchard Spirit — {2}{G} 2/2 Spirit.
/// Orchard Spirit can't be blocked except by creatures with flying or reach.
///
/// TODO: CantBeBlocked currently prevents ALL blocking. The real card allows
/// creatures with flying or reach to block it. This needs a new ContinuousEffect
/// variant (e.g., CantBeBlockedExceptBy) or a check in can_block_attacker that
/// respects flying/reach exceptions. For now, this is strictly stronger than
/// the real card.
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
                // TODO: Should be CantBeBlockedExceptByFlyingOrReach, not full CantBeBlocked.
                ContinuousEffect::CantBeBlocked { scope: EffectScope::OnSelf },
            ],
            triggered_abilities: vec![],
        }
    }
}

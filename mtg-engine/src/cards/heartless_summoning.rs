use crate::cards::{CardBehavior, CardData};
use crate::types::*;

/// Heartless Summoning — {1}{B} Enchantment.
/// Creature spells you cast cost {2} less to cast.
/// Creatures you control get -1/-1.
pub struct HeartlessSummoning;

impl CardBehavior for HeartlessSummoning {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Heartless Summoning".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Creature spells you cast cost {2} less to cast.\nCreatures you control get -1/-1.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: -1,
                    toughness: -1,
                    scope: EffectScope::Global(CreatureFilter::You),
                },
                ContinuousEffect::ReduceCost {
                    reduction: 2,
                    filter: SpellFilter::CreatureSpells,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }
}

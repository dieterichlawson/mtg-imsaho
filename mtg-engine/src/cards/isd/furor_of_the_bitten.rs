use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Furor of the Bitten — {R} aura enchantment. Enchanted creature gets +2/+2 and attacks each combat if able.
pub struct FurorOfTheBitten;

impl CardBehavior for FurorOfTheBitten {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Furor of the Bitten".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 2, toughness: 2, scope: EffectScope::Attached },
                ContinuousEffect::ForceAttack { scope: EffectScope::Attached },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets, registry);
    }
}

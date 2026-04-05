use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Bonds of Faith — {1}{W} aura enchantment.
/// Enchant creature.
/// Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
pub struct BondsOfFaith;

impl CardBehavior for BondsOfFaith {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bonds of Faith".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                // +2/+2 as long as attached creature is a Human.
                ContinuousEffect::ConditionalModifyPT {
                    power: 2,
                    toughness: 2,
                    condition: EffectCondition::AttachedHasSubtype("Human".into()),
                    scope: EffectScope::Attached,
                },
                // Can't attack as long as attached creature is NOT a Human.
                ContinuousEffect::ConditionalPreventAttack {
                    condition: EffectCondition::AttachedLacksSubtype("Human".into()),
                    scope: EffectScope::Attached,
                },
                // Can't block as long as attached creature is NOT a Human.
                ContinuousEffect::ConditionalPreventBlock {
                    condition: EffectCondition::AttachedLacksSubtype("Human".into()),
                    scope: EffectScope::Attached,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }
}

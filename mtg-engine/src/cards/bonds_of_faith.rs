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
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }

    fn dynamic_continuous_effects(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Option<Vec<ContinuousEffect>> {
        let target_id = state.get_object(object_id).and_then(|o| o.attached_to)?;
        let is_human = state.get_object(target_id)
            .map(|o| {
                o.subtypes.iter().any(|s| s == "Human")
                || registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if is_human {
            Some(vec![ContinuousEffect::ModifyPT { power: 2, toughness: 2, scope: EffectScope::Attached }])
        } else {
            Some(vec![
                ContinuousEffect::PreventAttack { scope: EffectScope::Attached },
                ContinuousEffect::PreventBlock { scope: EffectScope::Attached },
            ])
        }
    }
}

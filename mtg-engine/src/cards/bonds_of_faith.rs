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
            flashback_cost: None, continuous_effects: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        // Determine whether the enchanted creature is a Human.
        let target_id = state.get_object(object_id).and_then(|o| o.attached_to);
        if let Some(target_id) = target_id {
            let is_human = state.get_object(target_id)
                .and_then(|o| registry.card_data(o.card_id))
                .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                .unwrap_or(false);
            let instance_text = if is_human {
                "Enchanted creature gets +2/+2.".to_string()
            } else {
                "Enchanted creature can't attack or block.".to_string()
            };
            let target_name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            if is_human {
                state.log(crate::state::LogLevel::Event,
                    format!("Bonds of Faith gives {} +2/+2 (Human)", target_name));
            } else {
                state.log(crate::state::LogLevel::Event,
                    format!("Bonds of Faith prevents {} from attacking or blocking (non-Human)", target_name));
            }
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.instance_oracle_text = Some(instance_text);
            }
        }
    }
}

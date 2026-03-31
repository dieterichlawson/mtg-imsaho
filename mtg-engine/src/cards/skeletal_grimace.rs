use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, ActivatedAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skeletal Grimace — {1}{B} aura enchantment. Enchanted creature gets +1/+1 and has "{B}: Regenerate this creature."
pub struct SkeletalGrimace;

impl CardBehavior for SkeletalGrimace {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skeletal Grimace".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature gets +1/+1 and has \"{B}: Regenerate this creature.\"".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 1, scope: EffectScope::Attached },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }

    /// The aura grants "{B}: Regenerate" to the enchanted creature.
    /// `object_id` is the creature this is attached to.
    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        // Only grant regenerate to the enchanted creature (must have power, i.e. be a creature),
        // not to the aura itself.
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield && o.power.is_some()).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{B}: Regenerate".into(),
                cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Black)]),
                requires_tap: false,
                sacrifice_cost: crate::cards::SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    /// Apply the regeneration effect: add a regeneration shield.
    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.regeneration_shields += 1;
        }
    }
}

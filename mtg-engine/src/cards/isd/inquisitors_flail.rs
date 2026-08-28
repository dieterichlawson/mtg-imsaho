use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Zone};

/// Inquisitor's Flail — {2} Artifact — Equipment.
/// If equipped creature would deal combat damage, it deals double that damage instead.
/// If another source would deal combat damage to equipped creature, it deals double
/// that damage to equipped creature instead.
/// Equip {2}.
pub struct InquisitorsFlail;

impl CardBehavior for InquisitorsFlail {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Inquisitor's Flail".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "If equipped creature would deal combat damage, it deals double that damage instead.\nIf another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.\nEquip {2}".into(),
            continuous_effects: vec![
                ContinuousEffect::DoubleCombatDamage { scope: EffectScope::Attached },
            ],
            ..Default::default()
        }
    }


    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !state.is_creature(obj.id, registry) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {2}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
                once_per_turn: false,
                sorcery_speed_only: true,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }
}

use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Sharpened Pitchfork — {2} Artifact — Equipment.
/// Equipped creature has first strike.
/// As long as equipped creature is a Human, it gets +1/+1.
/// Equip {1}.
pub struct SharpenedPitchfork;

impl SharpenedPitchfork {
    fn update_effects(&self, state: &mut GameState, object_id: ObjectId, creature_id: ObjectId, registry: &CardRegistry) {
        let is_human = state.get_object(creature_id)
            .and_then(|o| registry.card_data(o.card_id))
            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
            .unwrap_or(false);

        let effects = if is_human {
            vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached },
                ContinuousEffect::ModifyPT { power: 1, toughness: 1, scope: EffectScope::Attached },
            ]
        } else {
            vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached },
            ]
        };

        if let Some(obj) = state.get_object_mut(object_id) {
            obj.instance_continuous_effects = Some(effects);
        }
    }
}

impl CardBehavior for SharpenedPitchfork {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sharpened Pitchfork".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Equipped creature has first strike. As long as equipped creature is a Human, it gets +1/+1.\nEquip {1}".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {1}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
                once_per_turn: false,
                sorcery_speed_only: true,
            }]
        } else {
            vec![]
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .map(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster)
                .unwrap_or(false),
            Target::Player(_) => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            let creature_id = *creature_id;
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(creature_id);
            }
            self.update_effects(state, object_id, creature_id, registry);
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }
}

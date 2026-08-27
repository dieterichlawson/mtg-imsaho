use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Keyword, Zone};

/// Mask of Avacyn — {2} Artifact — Equipment.
/// Equipped creature gets +1/+2 and has hexproof. Equip {3}.
pub struct MaskOfAvacyn;

impl CardBehavior for MaskOfAvacyn {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mask of Avacyn".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.)\nEquip {3}".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 2, scope: EffectScope::Attached },
                ContinuousEffect::GrantKeyword { keyword: Keyword::Hexproof, scope: EffectScope::Attached },
            ],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // Gate on power.is_none() — see Cobbled Wings for Bug AJ explanation.
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield && !state.is_creature(o.id, registry)) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {3}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(3)]),
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

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .is_some_and(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == caster),
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }

}

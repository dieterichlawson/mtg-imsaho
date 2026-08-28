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

    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }

}

use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Keyword};

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
        crate::cards::helpers::equip_for_generic(state, object_id, registry, 3)
    }

    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }

}

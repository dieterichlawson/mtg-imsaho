use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, Keyword, EffectScope};

/// Cobbled Wings — {2} Artifact — Equipment.
/// Equipped creature has flying. Equip {1}.
pub struct CobbledWings;

impl CardBehavior for CobbledWings {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Cobbled Wings".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature has flying.\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)".into(),
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::Flying, scope: EffectScope::Attached },
            ],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        crate::cards::helpers::equip_for_generic(state, object_id, registry, 1)
    }

    /// Equip targets only creatures you control.
    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }

}

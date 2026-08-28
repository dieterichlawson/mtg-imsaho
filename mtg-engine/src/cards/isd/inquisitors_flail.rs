use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope};

/// Inquisitor's Flail — {2} Artifact — Equipment.
/// If equipped creature would deal combat damage, it deals double that damage instead.
/// If another creature would deal combat damage to equipped creature, it deals double
/// that damage to equipped creature instead.
/// Equip {2}
///
/// The second clause says "another **creature**", not "another source"; this
/// comment used to say the latter. Nothing in this set deals combat damage
/// from a noncreature source — only attacking and blocking creatures deal
/// combat damage at all — so the two readings never diverge here, but the
/// card does not say "source" and the comment should not either.
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
        crate::cards::helpers::equip_for_generic(state, object_id, registry, 2)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }
}

use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, EffectCondition};

/// Silver-Inlaid Dagger — {1} Artifact — Equipment.
/// Equipped creature gets +2/+0.
/// As long as equipped creature is a Human, it gets an additional +1/+0.
/// Equip {2}.
///
/// Implementation notes:
/// The Human bonus is a *continuous* conditional effect, not a snapshot taken
/// at equip time. We use `ContinuousEffect::when` with `ModifyPT` (the same pattern
/// Bonds of Faith uses) so that if the equipped creature transforms (e.g. a
/// Human Werewolf flips into its non-Human back face via Moonmist or via a
/// no-spells-last-turn upkeep trigger), the +1/+0 drops in real time. Likewise
/// if a non-Human ever gains the Human subtype, the +1/+0 appears.
pub struct SilverInlaidDagger;

impl CardBehavior for SilverInlaidDagger {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Silver-Inlaid Dagger".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1)])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature gets +2/+0.\nAs long as equipped creature is a Human, it gets an additional +1/+0.\nEquip {2}".into(),
            continuous_effects: vec![
                // Unconditional +2/+0 to the equipped creature.
                ContinuousEffect::ModifyPT { power: 2, toughness: 0, scope: EffectScope::Attached },
                // Additional +1/+0 only while the equipped creature is a Human.
                // Re-evaluated every time effective P/T is computed, so transform
                // triggers and type-changing effects update the bonus correctly.
                ContinuousEffect::when(
                    EffectCondition::AttachedHasSubtype("Human".into()),
                    ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Attached },
                ),
            ],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        crate::cards::helpers::equip_for_generic(state, object_id, registry, 2)
    }

    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }

}

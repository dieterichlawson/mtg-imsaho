use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Keyword, EffectCondition};

/// Butcher's Cleaver — {3} Artifact — Equipment.
/// Equipped creature gets +3/+0.
/// As long as equipped creature is a Human, it has lifelink.
/// Equip {3}.
///
/// Implementation notes:
/// The Human-conditional lifelink is a *continuous* conditional effect, not a
/// snapshot taken at equip time. We use `ContinuousEffect::when` with `GrantKeyword`
/// (the same pattern Bonds of Faith uses for its conditional bonuses) so that
/// if the equipped creature transforms (e.g. a Human Werewolf flips into its
/// non-Human back face via Moonmist), lifelink drops in real time.
pub struct ButchersCleaver;

impl CardBehavior for ButchersCleaver {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Butcher's Cleaver".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3)])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature gets +3/+0.\nAs long as equipped creature is a Human, it has lifelink.\nEquip {3}".into(),
            continuous_effects: vec![
                // Unconditional +3/+0 to the equipped creature.
                ContinuousEffect::ModifyPT { power: 3, toughness: 0, scope: EffectScope::Attached },
                // Lifelink only while the equipped creature is a Human. Re-evaluated
                // continuously so transform triggers and type-changing effects update
                // the bonus correctly.
                ContinuousEffect::when(
                    EffectCondition::AttachedHasSubtype("Human".into()),
                    ContinuousEffect::GrantKeyword { keyword: Keyword::Lifelink, scope: EffectScope::Attached },
                ),
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

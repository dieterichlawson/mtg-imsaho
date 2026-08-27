use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, Keyword, EffectScope, EffectCondition, Zone};

/// Sharpened Pitchfork — {2} Artifact — Equipment.
/// Equipped creature has first strike.
/// As long as equipped creature is a Human, it gets +1/+1.
/// Equip {1}.
///
/// Implementation notes:
/// The Human-conditional +1/+1 is a *continuous* conditional effect, not a
/// snapshot taken at equip time. We use `ContinuousEffect::when` with `ModifyPT`
/// (the same pattern Bonds of Faith and Silver-Inlaid Dagger use) so that if
/// the equipped creature transforms (e.g. a Human Werewolf flips into its
/// non-Human back face via Moonmist), the +1/+1 drops in real time.
pub struct SharpenedPitchfork;

impl CardBehavior for SharpenedPitchfork {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sharpened Pitchfork".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature has first strike.\nAs long as equipped creature is a Human, it gets +1/+1.\nEquip {1}".into(),
            continuous_effects: vec![
                // Unconditional first strike to the equipped creature.
                ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached },
                // Additional +1/+1 only while the equipped creature is a Human.
                // Re-evaluated continuously so transform triggers update the bonus.
                ContinuousEffect::when(
                    EffectCondition::AttachedHasSubtype("Human".into()),
                    ContinuousEffect::ModifyPT { power: 1, toughness: 1, scope: EffectScope::Attached },
                ),
            ],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // Gate on power.is_none() — see Cobbled Wings for Bug AJ explanation.
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield && !state.is_creature(o.id, registry)) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {1}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
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

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }
}

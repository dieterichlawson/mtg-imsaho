use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Keyword, EffectCondition, Zone};

/// Butcher's Cleaver — {3} Artifact — Equipment.
/// Equipped creature gets +3/+0.
/// As long as equipped creature is a Human, it has lifelink.
/// Equip {3}.
///
/// Implementation notes:
/// The Human-conditional lifelink is a *continuous* conditional effect, not a
/// snapshot taken at equip time. We use ContinuousEffect::ConditionalKeyword
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
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Equipped creature gets +3/+0.\nAs long as equipped creature is a Human, it has lifelink.\nEquip {3}".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                // Unconditional +3/+0 to the equipped creature.
                ContinuousEffect::ModifyPT { power: 3, toughness: 0, scope: EffectScope::Attached },
                // Lifelink only while the equipped creature is a Human. Re-evaluated
                // continuously so transform triggers and type-changing effects update
                // the bonus correctly.
                ContinuousEffect::ConditionalKeyword {
                    keyword: Keyword::Lifelink,
                    condition: EffectCondition::AttachedHasSubtype("Human".into()),
                    scope: EffectScope::Attached,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // Gate on power.is_none() — see Cobbled Wings for Bug AJ explanation.
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield && o.power.is_none()) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {3}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(3)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
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
                .is_some_and(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster),
            Target::Player(_) => false,
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

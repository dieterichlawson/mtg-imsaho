use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement, TriggeredAbilityDef, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Zone};

/// Wooden Stake — {2} Artifact — Equipment.
/// Equipped creature gets +1/+0.
/// Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that Vampire.
/// Equip {1}.
pub struct WoodenStake;

impl CardBehavior for WoodenStake {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Wooden Stake".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Equipped creature gets +1/+0.\nWhenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Attached },
            ],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Blocks,
                    description: "destroy that Vampire".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::BecomesBlocked,
                    description: "destroy that Vampire".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // Gate on power.is_none() — see Cobbled Wings for Bug AJ explanation.
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield && o.power.is_none()) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {1}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
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

    fn on_blocks(&self, state: &mut GameState, _self_id: ObjectId, other_creature: ObjectId, registry: &CardRegistry) {
        // Check if the other creature is a Vampire.
        let is_vampire = state.get_object(other_creature)
            .and_then(|o| registry.card_data(o.card_id))
            .is_some_and(|d| d.subtypes.iter().any(|s| s == "Vampire"));

        // Also check instance subtypes on the game object (for tokens, etc.).
        let is_vampire = is_vampire || state.get_object(other_creature)
            .is_some_and(|o| o.subtypes.iter().any(|s| s == "Vampire"));

        if is_vampire {
            state.log(crate::state::LogLevel::Event, format!("Wooden Stake destroys {} (Vampire)", state.obj_name(other_creature)));
            crate::destruction::try_destroy_no_regen(state, other_creature, registry);
        }
    }

    fn on_becomes_blocked(&self, state: &mut GameState, _self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) {
        // Same check: if the blocker is a Vampire, destroy it.
        let is_vampire = state.get_object(blocker_id)
            .and_then(|o| registry.card_data(o.card_id))
            .is_some_and(|d| d.subtypes.iter().any(|s| s == "Vampire"))
            || state.get_object(blocker_id)
                .is_some_and(|o| o.subtypes.iter().any(|s| s == "Vampire"));
        if is_vampire {
            state.log(crate::state::LogLevel::Event, format!("Wooden Stake destroys {} (Vampire)", state.obj_name(blocker_id)));
            crate::destruction::try_destroy_no_regen(state, blocker_id, registry);
        }
    }
}

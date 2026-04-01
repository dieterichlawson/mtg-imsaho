use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement, TriggeredAbilityDef, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            oracle_text: "Equipped creature gets +1/+0.\nWhenever equipped creature blocks or becomes blocked by a Vampire, destroy that Vampire.\nEquip {1}".into(),
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
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::BecomesBlocked,
                    description: "destroy that Vampire".into(),
                },
            ],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {1}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
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
                .map(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster)
                .unwrap_or(false),
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

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }

    fn on_blocks(&self, state: &mut GameState, _self_id: ObjectId, other_creature: ObjectId, registry: &CardRegistry) {
        // Check if the other creature is a Vampire.
        let is_vampire = state.get_object(other_creature)
            .and_then(|o| registry.card_data(o.card_id))
            .map(|d| d.subtypes.iter().any(|s| s == "Vampire"))
            .unwrap_or(false);

        // Also check instance subtypes on the game object (for tokens, etc.).
        let is_vampire = is_vampire || state.get_object(other_creature)
            .map(|o| o.subtypes.iter().any(|s| s == "Vampire"))
            .unwrap_or(false);

        if is_vampire {
            let name = state.get_object(other_creature).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event, format!("Wooden Stake destroys {} (Vampire)", name));
            crate::destruction::try_destroy_no_regen(state, other_creature, registry);
        }
    }

    fn on_becomes_blocked(&self, state: &mut GameState, _self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) {
        // Same check: if the blocker is a Vampire, destroy it.
        let is_vampire = state.get_object(blocker_id)
            .and_then(|o| registry.card_data(o.card_id))
            .map(|d| d.subtypes.iter().any(|s| s == "Vampire"))
            .unwrap_or(false)
            || state.get_object(blocker_id)
                .map(|o| o.subtypes.iter().any(|s| s == "Vampire"))
                .unwrap_or(false);
        if is_vampire {
            let name = state.get_object(blocker_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event, format!("Wooden Stake destroys {} (Vampire)", name));
            crate::destruction::try_destroy_no_regen(state, blocker_id, registry);
        }
    }
}

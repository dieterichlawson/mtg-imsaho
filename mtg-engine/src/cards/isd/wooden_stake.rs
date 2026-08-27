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
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature gets +1/+0.\nWhenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Attached },
            ],
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

    // "Whenever equipped creature blocks or becomes blocked by a Vampire" —
    // the Vampire condition is part of the trigger itself (CR 603.2), so it
    // gates dispatch; the resolution handlers re-check for defense in depth.
    fn should_trigger_on_blocks(&self, state: &GameState, _self_id: ObjectId, blocked_attacker: ObjectId, registry: &CardRegistry) -> bool {
        state.has_subtype(blocked_attacker, "Vampire", registry)
    }

    fn should_trigger_on_becomes_blocked(&self, state: &GameState, _self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) -> bool {
        state.has_subtype(blocker_id, "Vampire", registry)
    }

    fn on_blocks(&self, state: &mut GameState, _self_id: ObjectId, other_creature: ObjectId, registry: &CardRegistry) {
        if state.has_subtype(other_creature, "Vampire", registry) {
            state.log(crate::state::LogLevel::Event, format!("Wooden Stake destroys {} (Vampire)", state.obj_name(other_creature)));
            crate::destruction::try_destroy_no_regen(state, other_creature, registry);
        }
    }

    fn on_becomes_blocked(&self, state: &mut GameState, _self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) {
        if state.has_subtype(blocker_id, "Vampire", registry) {
            state.log(crate::state::LogLevel::Event, format!("Wooden Stake destroys {} (Vampire)", state.obj_name(blocker_id)));
            crate::destruction::try_destroy_no_regen(state, blocker_id, registry);
        }
    }
}

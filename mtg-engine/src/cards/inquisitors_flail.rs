use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Inquisitor's Flail — {2} Artifact — Equipment.
/// If equipped creature would deal combat damage, it deals double that damage instead.
/// If another source would deal combat damage to equipped creature, it deals double
/// that damage to equipped creature instead.
/// Equip {2}.
///
/// SIMPLIFICATION: The double-damage replacement effect is complex to implement in the
/// combat system. Instead, we approximate the offensive half by granting a +P/+0 bonus
/// equal to the creature's effective power (effectively doubling its power for combat).
/// The defensive doubling (taking double combat damage) is not implemented.
pub struct InquisitorsFlail;

impl CardBehavior for InquisitorsFlail {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Inquisitor's Flail".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "If equipped creature would deal combat damage, it deals double that damage instead. If another source would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.\nEquip {2}".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Approximate the offensive double damage by granting +P/+0 equal to the creature's
    /// base effective power (before this bonus). This effectively doubles power for combat.
    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        let obj = state.get_object(object_id)?;
        if obj.zone != Zone::Battlefield {
            return None;
        }
        // Get the attached creature's ID.
        let creature_id = obj.attached_to?;
        let creature = state.get_object(creature_id)?;
        if creature.zone != Zone::Battlefield {
            return None;
        }
        // Use the creature's base power (from the object, without continuous effects from this equipment).
        // This is an approximation — we use the raw power value which includes P/T modifications
        // from other sources but not this equipment's own contribution.
        let base_power = creature.power.unwrap_or(0);
        // Add +1/+1 counters.
        let counter_bonus = *creature.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0) as i32;
        let counter_penalty = *creature.counters.get(&CounterType::MinusOneMinusOne).unwrap_or(&0) as i32;
        let effective = base_power + counter_bonus - counter_penalty;
        // Until-end-of-turn effects on the creature.
        let eot_bonus: i32 = state.until_end_of_turn_effects.iter()
            .filter(|e| e.target == creature_id)
            .map(|e| e.power_mod)
            .sum();
        let total_power = (effective + eot_bonus).max(0);
        Some((total_power, 0))
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && obj.power.is_none() {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {2}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
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

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }
}

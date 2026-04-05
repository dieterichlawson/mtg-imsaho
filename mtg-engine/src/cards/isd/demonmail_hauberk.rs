use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Demonmail Hauberk — {4} Artifact — Equipment.
/// Equipped creature gets +4/+2.
/// Equip—Sacrifice a creature.
pub struct DemonmailHauberk;

impl CardBehavior for DemonmailHauberk {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Demonmail Hauberk".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Equipped creature gets +4/+2.\nEquip—Sacrifice a creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 4, toughness: 2, scope: EffectScope::Attached },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // Equip can only target creatures you control.
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        // Equip ability is on the equipment itself (no power = not a creature).
        if obj.zone == Zone::Battlefield && obj.power.is_none() {
            let controller = obj.controller;
            // The equip cost is "Sacrifice a creature." After paying this cost, there
            // must still be a creature to equip to. Require at least 2 creatures: one
            // to sacrifice as the cost, and one remaining to be the equip target.
            let creature_count = state.objects_in_zone(Zone::Battlefield, controller)
                .iter()
                .filter(|o| o.power.is_some())
                .count();
            if creature_count < 2 {
                return vec![];
            }
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip—Sacrifice a creature".into(),
                cost: ManaCost::free(),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::SacrificeCreature,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
                once_per_turn: false,
                sorcery_speed_only: true,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        // Attach equipment to the target creature.
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }
}

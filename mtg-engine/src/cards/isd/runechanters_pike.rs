use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Runechanter's Pike — {2} Artifact — Equipment.
/// Equipped creature has first strike and gets +X/+0, where X is the number
/// of instant and sorcery cards in your graveyard.
/// Equip {2}.
pub struct RunechantersPike;

impl CardBehavior for RunechantersPike {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Runechanter's Pike".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.\nEquip {2}".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
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

    /// Dynamic P/T: +X/+0 where X = instant/sorcery count in controller's graveyard.
    /// Called by the engine when computing P/T for the attached creature.
    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        let obj = state.get_object(object_id)?;
        if obj.zone != Zone::Battlefield {
            return None;
        }
        let controller = obj.controller;
        let count = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller)
            .filter(|o| {
                o.card_types.contains(&CardType::Instant) || o.card_types.contains(&CardType::Sorcery)
            })
            .count() as i32;
        Some((count, 0))
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        // Equip ability on the equipment itself.
        if obj.zone == Zone::Battlefield && obj.power.is_none() {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {2}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
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

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }
}

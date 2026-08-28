use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, Keyword, EffectScope, Zone};

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
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.\nEquip {2}".into(),
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached },
            ],
            ..Default::default()
        }
    }


    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    /// Dynamic P/T: +X/+0 where X = instant/sorcery count in controller's graveyard.
    /// Called by the engine when computing P/T for the attached creature.
    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Option<(i32, i32)> {
        let obj = state.get_object(object_id)?;
        if obj.zone != Zone::Battlefield {
            return None;
        }
        let controller = obj.controller;
        let count = i32::try_from(state.objects_in_zone(Zone::Graveyard, controller).into_iter()
            .filter(|o| state.is_card(o.id))
            .filter(|o| state.has_card_type(o.id, CardType::Instant, registry)
                || state.has_card_type(o.id, CardType::Sorcery, registry))
            .count()).unwrap_or(i32::MAX);
        Some((count, 0))
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        // Equip ability on the equipment itself.
        if obj.zone == Zone::Battlefield && !state.is_creature(obj.id, registry) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {2}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
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

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }
}

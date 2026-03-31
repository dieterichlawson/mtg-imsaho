use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Daybreak Ranger {2}{G} 2/2 Human Archer Werewolf — {T}: deal 2 to flying creature
/// // Nightfall Predator 4/4 Werewolf — {R},{T}: fight target creature
pub struct DaybreakRanger;

impl DaybreakRanger {
    fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
        let is_transformed = state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false);
        let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();
        if !is_transformed {
            total_spells_last_turn == 0 && !state.is_first_turn
        } else {
            state.spells_cast_last_turn.values().any(|&count| count >= 2)
        }
    }
}

impl CardBehavior for DaybreakRanger {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Daybreak Ranger".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Archer".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{T}: Daybreak Ranger deals 2 damage to target creature with flying.\nAt the beginning of each upkeep, if no spells were cast last turn, transform Daybreak Ranger.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Nightfall Predator".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "{R}, {T}: Nightfall Predator fights target creature.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Nightfall Predator.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        Self::werewolf_should_transform(state, object_id)
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((4, 4))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        if obj.is_transformed {
            // Nightfall Predator: {R}, {T}: fight target creature
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{R}, {T}: Fight target creature".into(),
                cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            // Daybreak Ranger: {T}: deal 2 to creature with flying
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{T}: Deal 2 damage to target creature with flying".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield && o.power.is_some() => o,
                    _ => return false,
                };
                // Check if the source is transformed (Nightfall Predator targets any creature you don't control).
                let self_transformed = state.objects.values()
                    .find(|o| o.controller == caster && o.zone == Zone::Battlefield
                        && registry.card_data(o.card_id).map(|d| d.name == "Daybreak Ranger").unwrap_or(false))
                    .map(|o| o.is_transformed)
                    .unwrap_or(false);
                if self_transformed {
                    // Nightfall Predator: fight any creature you don't control
                    obj.controller != caster
                } else {
                    // Daybreak Ranger: target creature with flying
                    state.has_keyword(*id, Keyword::Flying, registry)
                }
            }
            Target::Player(_) => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let is_transformed = state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false);
        if let Some(Target::Object(target_id)) = targets.first() {
            if is_transformed {
                // Nightfall Predator: fight
                crate::combat::fight(state, object_id, *target_id, registry);
            } else {
                // Daybreak Ranger: deal 2 damage to creature with flying
                let target_name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
                if let Some(obj) = state.get_object_mut(*target_id) {
                    if obj.zone == Zone::Battlefield {
                        obj.damage_marked += 2;
                        obj.damaged_by.push(object_id);
                    }
                }
                state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                    source: object_id,
                    target: crate::events::DamageTarget::Object(*target_id),
                    amount: 2,
                });
                state.log(crate::state::LogLevel::Event,
                    format!("Daybreak Ranger deals 2 damage to {}", target_name));
            }
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = !obj.is_transformed;
                let name = if obj.is_transformed { "Nightfall Predator" } else { "Daybreak Ranger" };
                obj.name = name.into();
                state.log(crate::state::LogLevel::Event,
                    format!("Daybreak Ranger transforms into {}", name));
            }
        }
    }
}

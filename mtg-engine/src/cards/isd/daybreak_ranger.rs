use crate::actions::Target;
use crate::cards::helpers;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Daybreak Ranger {2}{G} 2/2 Human Archer Werewolf — {T}: this creature deals 2 to flying creature
/// // Nightfall Predator 4/4 Werewolf — {R},{T}: this creature fights target creature
pub struct DaybreakRanger;


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
            subtypes: vec!["Human".into(), "Archer".into(), "Ranger".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{T}: This creature deals 2 damage to target creature with flying.\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                target_requirement: None,
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
            oracle_text: "{R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform back if 2+ spells cast".into(),
                target_requirement: None,
                },
            ],
        })
    }

    fn should_trigger(&self, state: &GameState, self_id: ObjectId, kind: &TriggerKind, registry: &CardRegistry) -> bool {
        helpers::werewolf_should_trigger(self, state, self_id, kind, registry)
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        helpers::werewolf_should_transform(state, object_id)
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Option<(i32, i32)> {
        if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
            Some((4, 4))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
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
                counter_cost: None,
            }]
        } else {
            // Daybreak Ranger: {T}: deal 2 to creature with flying
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{T}: Deal 2 damage to target creature with flying".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::HasKeyword(Keyword::Flying))),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let is_transformed = state.get_object(object_id).is_some_and(|o| o.is_transformed);
        if let Some(Target::Object(target_id)) = targets.first() {
            if is_transformed {
                // Nightfall Predator: fight
                crate::combat::fight(state, object_id, *target_id, registry);
            } else {
                // Daybreak Ranger: deal 2 damage to creature with flying
                let effect = crate::state::PendingEffect::DealDamage {
                    amount: 2,
                    source_id: object_id,
                    source_name: "Daybreak Ranger".into(),
                };
                crate::engine::apply_pending_effect(
                    state,
                    &Target::Object(*target_id),
                    &effect,
                    registry,
                );
            }
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            let old_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
            helpers::apply_transform(state, self_id, registry);
            let new_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("{old_name} transforms into {new_name}"));
        }
    }
}

use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Ulvenwald Mystics {2}{G}{G} 3/3 Human Shaman Werewolf
/// // Ulvenwald Primordials 5/5 Werewolf with {G}: Regenerate
pub struct UlvenwaldMystics;

impl UlvenwaldMystics {
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

impl CardBehavior for UlvenwaldMystics {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ulvenwald Mystics".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Shaman".into(), "Werewolf".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "At the beginning of each upkeep, if no spells were cast last turn, transform Ulvenwald Mystics.".into(),
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
            name: "Ulvenwald Primordials".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "{G}: Regenerate Ulvenwald Primordials.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Ulvenwald Primordials.".into(),
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
            Some((5, 5))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield && o.is_transformed => o,
            _ => return vec![],
        };
        let _ = obj;
        // Ulvenwald Primordials: {G}: Regenerate
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{G}: Regenerate".into(),
            cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.regeneration_shields += 1;
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = !obj.is_transformed;
                let name = if obj.is_transformed { "Ulvenwald Primordials" } else { "Ulvenwald Mystics" };
                obj.name = name.into();
                state.log(crate::state::LogLevel::Event,
                    format!("Ulvenwald Mystics transforms into {}", name));
            }
        }
    }
}

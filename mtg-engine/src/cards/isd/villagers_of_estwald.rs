use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Villagers of Estwald {2}{G} 2/3 Human Werewolf // Howlpack of Estwald 4/6 Werewolf
pub struct VillagersOfEstwald;

impl VillagersOfEstwald {
    fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
        let is_transformed = state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false);
        let total_spells_last_turn: u32 = state.num_spells_cast_last_turn.values().sum();
        if !is_transformed {
            total_spells_last_turn == 0 && !state.is_first_turn
        } else {
            state.num_spells_cast_last_turn.values().any(|&count| count >= 2)
        }
    }
}

impl CardBehavior for VillagersOfEstwald {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Villagers of Estwald".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "At the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
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
            name: "Howlpack of Estwald".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(4),
            toughness: Some(6),
            oracle_text: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack of Estwald.".into(),
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
            Some((4, 6))
        } else {
            None
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = !obj.is_transformed;
                let name = if obj.is_transformed { "Howlpack of Estwald" } else { "Villagers of Estwald" };
                obj.name = name.into();
                state.log(crate::state::LogLevel::Event,
                    format!("Villagers of Estwald transforms into {}", name));
            }
        }
    }
}

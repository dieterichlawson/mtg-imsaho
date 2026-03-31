use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Grizzled Outcasts {4}{G} 4/4 Human Werewolf // Krallenhorde Wantons 7/7 Werewolf
pub struct GrizzledOutcasts;

impl GrizzledOutcasts {
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

impl CardBehavior for GrizzledOutcasts {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grizzled Outcasts".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Werewolf".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "At the beginning of each upkeep, if no spells were cast last turn, transform Grizzled Outcasts.".into(),
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
            name: "Krallenhorde Wantons".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(7),
            toughness: Some(7),
            oracle_text: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Krallenhorde Wantons.".into(),
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
            Some((7, 7))
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
                let name = if obj.is_transformed { "Krallenhorde Wantons" } else { "Grizzled Outcasts" };
                obj.name = name.into();
                state.log(crate::state::LogLevel::Event,
                    format!("Grizzled Outcasts transforms into {}", name));
            }
        }
    }
}

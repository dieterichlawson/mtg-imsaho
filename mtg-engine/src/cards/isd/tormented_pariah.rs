use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Tormented Pariah {3}{R} 3/2 Human Warrior // Rampaging Werewolf 6/4 Werewolf
pub struct TormentedPariah;

impl TormentedPariah {
    fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
        let is_transformed = state.get_object(object_id).is_some_and(|o| o.is_transformed);
        let total_spells_last_turn: u32 = state.num_spells_cast_last_turn.values().sum();
        if is_transformed {
            state.num_spells_cast_last_turn.values().any(|&count| count >= 2)
        } else {
            total_spells_last_turn == 0 && !state.is_first_turn
        }
    }
}

impl CardBehavior for TormentedPariah {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Tormented Pariah".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Warrior".into(), "Werewolf".into()],
            power: Some(3),
            toughness: Some(2),
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
            name: "Rampaging Werewolf".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(6),
            toughness: Some(4),
            oracle_text: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Rampaging Werewolf.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform back if 2+ spells cast".into(),
                },
            ],
        })
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        Self::werewolf_should_transform(state, object_id)
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
            Some((6, 4))
        } else {
            None
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
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

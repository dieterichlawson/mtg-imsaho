use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Kruin Outlaw {1}{R}{R} 2/2 Human Rogue Werewolf with First Strike
/// // Terror of Kruin Pass 3/3 Werewolf with Double Strike + menace
pub struct KruinOutlaw;

impl KruinOutlaw {
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

impl CardBehavior for KruinOutlaw {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kruin Outlaw".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Rogue".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "First strike\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            keywords: vec![Keyword::FirstStrike],
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
            name: "Terror of Kruin Pass".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Double strike\nWerewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
            keywords: vec![Keyword::DoubleStrike],
            flashback_cost: None,
            continuous_effects: vec![
                // "Werewolves you control have menace."
                ContinuousEffect::GrantKeyword {
                    keyword: Keyword::Menace,
                    scope: EffectScope::Global(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::HasSubtype("Werewolf".into()),
                    ])),
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                },
            ],
        })
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> bool {
        Self::werewolf_should_transform(state, object_id)
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((3, 3))
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
                let name = if obj.is_transformed { "Terror of Kruin Pass" } else { "Kruin Outlaw" };
                obj.name = name.into();
                state.log(crate::state::LogLevel::Event,
                    format!("Kruin Outlaw transforms into {}", name));
            }
        }
    }
}

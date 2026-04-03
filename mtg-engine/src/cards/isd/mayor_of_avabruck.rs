use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Mayor of Avabruck {1}{G} 1/1 Human Advisor — other Humans +1/+1
/// // Howlpack Alpha 3/3 Werewolf — other Werewolves and Wolves +1/+1, EOT create 2/2 Wolf
pub struct MayorOfAvabruck;

impl MayorOfAvabruck {
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

impl CardBehavior for MayorOfAvabruck {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mayor of Avabruck".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Advisor".into(), "Werewolf".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Other Human creatures you control get +1/+1.\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 1,
                    toughness: 1,
                    scope: EffectScope::GlobalOther(CreatureFilter::And(vec![
                        CreatureFilter::You,
                        CreatureFilter::HasSubtype("Human".into()),
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
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Howlpack Alpha".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Each other creature you control that's a Werewolf or a Wolf gets +1/+1.\nAt the beginning of your end step, create a 2/2 green Wolf creature token.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 1,
                    toughness: 1,
                    scope: EffectScope::GlobalOther(CreatureFilter::And(vec![
                        CreatureFilter::You,
                        CreatureFilter::Or(vec![
                            CreatureFilter::HasSubtype("Werewolf".into()),
                            CreatureFilter::HasSubtype("Wolf".into()),
                        ]),
                    ])),
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform if a player cast 2+ spells last turn".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "create a 2/2 Wolf token".into(),
                },
            ],
        })
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
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
                let name = if obj.is_transformed { "Howlpack Alpha" } else { "Mayor of Avabruck" };
                obj.name = name.into();
                state.log(crate::state::LogLevel::Event,
                    format!("Mayor of Avabruck transforms into {}", name));
            }
        }
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        // Only create Wolf tokens on the back face (Howlpack Alpha) during controller's end step.
        if !is_transformed || state.active_player != controller {
            return;
        }
        state.create_token_with_subtypes(
            "Wolf",
            controller,
            2, 2,
            vec![Color::Green],
            vec![CardType::Creature],
            vec![],
            vec!["Wolf".into()],
        );
        state.log(crate::state::LogLevel::Event,
            "Howlpack Alpha: created a 2/2 Wolf token".into());
    }
}

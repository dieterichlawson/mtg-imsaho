use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::cards::helpers;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Screeching Bat {2}{B} 2/2 Bat with Flying // Stalking Vampire 5/5 Vampire.
/// Both faces: "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform."
pub struct ScreechingBat;

impl ScreechingBat {
    fn transform_cost() -> ManaCost {
        ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Black),
            ManaSymbol::Colored(Color::Black),
        ])
    }
}

impl CardBehavior for ScreechingBat {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Screeching Bat".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Bat".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "you may pay {2}{B}{B} to transform".into(),
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Stalking Vampire".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "you may pay {2}{B}{B} to transform".into(),
                },
            ],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((5, 5))
        } else {
            None
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // "You may pay {2}{B}{B}. If you do, transform."
        // Only present the choice if the player can afford the cost.
        let cost = Self::transform_cost();
        let pool = &state.get_player(controller).mana_pool;
        if !crate::mana::can_pay(pool, &cost) {
            return;
        }

        let current_name = state.get_object(self_id)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "this creature".into());

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: format!("{}: pay {{2}}{{B}}{{B}} to transform?", current_name),
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            let current_name = state.get_object(self_id)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "this creature".into());
            state.log(LogLevel::Event,
                format!("{}: chose not to pay", current_name));
            return;
        }

        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Pay the cost.
        let cost = Self::transform_cost();
        if crate::mana::auto_pay(&mut state.get_player_mut(controller).mana_pool, &cost).is_err() {
            // Can't pay — shouldn't happen since we checked in on_upkeep, but handle gracefully.
            state.log(LogLevel::Event, "Could not pay {2}{B}{B} to transform".into());
            return;
        }

        // Transform — uses the generic helper to update name, keywords, and subtypes.
        helpers::apply_transform(state, self_id, registry);
        let new_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
        state.log(LogLevel::Event,
            format!("Transforms into {}", new_name));
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, registry: &CardRegistry) -> bool {
        false
    }
}

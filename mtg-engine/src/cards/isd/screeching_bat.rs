use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Screeching Bat {2}{B} 2/2 Bat with Flying // Stalking Vampire 5/5 Vampire.
/// Both faces: "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform."
pub struct ScreechingBat;

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
            oracle_text: "Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Screeching Bat.".into(),
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
            oracle_text: "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Stalking Vampire.".into(),
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

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // "You may pay {2}{B}{B}. If you do, transform."
        // Auto-pay if the controller has enough mana (simplified "you may").
        let pool = &state.get_player(controller).mana_pool;
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Black),
            ManaSymbol::Colored(Color::Black),
        ]);
        if crate::mana::can_pay(pool, &cost) {
            crate::mana::auto_pay(&mut state.get_player_mut(controller).mana_pool, &cost).ok();
            let is_transformed = state.get_object(self_id).map(|o| o.is_transformed).unwrap_or(false);
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = !is_transformed;
                let name = if obj.is_transformed { "Stalking Vampire" } else { "Screeching Bat" };
                obj.name = name.into();
            }
            let new_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("Transforms into {}", new_name));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

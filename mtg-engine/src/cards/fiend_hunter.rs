use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Fiend Hunter — {1}{W}{W} 1/3 Human Cleric.
/// When Fiend Hunter enters the battlefield, you may exile another target creature.
/// When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield
/// under its owner's control.
pub struct FiendHunter;

impl CardBehavior for FiendHunter {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Fiend Hunter".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(3),
            oracle_text: "When Fiend Hunter enters the battlefield, you may exile another target creature. When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "exile another target creature".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::LeavesBattlefield,
                    description: "return exiled card to the battlefield".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        // Find opponent's strongest creature.
        let target = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller != controller && o.power.is_some() && o.id != object_id)
            .max_by_key(|o| o.power.unwrap_or(0))
            .map(|o| o.id);
        if let Some(target_id) = target {
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(target_id, Zone::Exile);
            // Store exiled creature ID in card_state for retrieval on LTB.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.card_state.insert("exiled_creature".into(), target_id);
            }
            state.log(crate::state::LogLevel::Event, format!("Fiend Hunter exiled {}", name));
        }
    }

    fn on_leave_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        // Retrieve the exiled creature's ID from card_state.
        let exiled_id = state.get_object(object_id)
            .and_then(|o| o.card_state.get("exiled_creature").copied());
        if let Some(target_id) = exiled_id {
            if state.get_object(target_id).map(|o| o.zone == Zone::Exile).unwrap_or(false) {
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(target_id, Zone::Battlefield);
                state.log(crate::state::LogLevel::Event, format!("{} returned to the battlefield", name));
            }
        }
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Splinterfright — {2}{G} */* Elemental. Trample.
/// Splinterfright's power and toughness are each equal to the number of
/// creature cards in your graveyard.
/// At the beginning of your upkeep, mill two cards.
pub struct Splinterfright;

impl CardBehavior for Splinterfright {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Splinterfright".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Elemental".into()],
            // */* — the CDA (dynamic_pt) defines actual P/T. Some(0) is needed so
            // the engine recognizes this as a creature (power.is_some() is used as proxy).
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Trample\nSplinterfright's power and toughness are each equal to the number of creature cards in your graveyard.\nAt the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "mill two cards".into(),
                },
            ],
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        let controller = state.get_object(object_id)?.controller;
        let creature_cards_in_gy = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .count() as i32;
        Some((creature_cards_in_gy, creature_cards_in_gy))
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Only trigger on your upkeep.
        if state.active_player != controller {
            return;
        }
        crate::engine::mill_cards(state, controller, 2, registry);
        state.log(crate::state::LogLevel::Event, "Splinterfright: milled 2 cards".into());
    }
}

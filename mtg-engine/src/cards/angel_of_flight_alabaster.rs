use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Angel of Flight Alabaster — {4}{W} 4/4 flying Angel.
/// At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
pub struct AngelOfFlightAlabaster;

impl CardBehavior for AngelOfFlightAlabaster {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Angel of Flight Alabaster".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Angel".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Flying\nAt the beginning of your upkeep, return target Spirit card from your graveyard to your hand.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "return target Spirit card from your graveyard to your hand".into(),
                },
            ],
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
        // Find a Spirit card in graveyard and return it to hand.
        let spirit = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .find(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Spirit"))
                    .unwrap_or(false)
                || o.subtypes.iter().any(|s| s == "Spirit")
            })
            .map(|o| o.id);
        if let Some(spirit_id) = spirit {
            let name = state.get_object(spirit_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(spirit_id, Zone::Hand);
            state.log(crate::state::LogLevel::Event,
                format!("Angel of Flight Alabaster: returned {} to hand", name));
        }
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Dearly Departed — {4}{W}{W} 5/5 Spirit with Flying.
/// As long as this creature is in your graveyard, each Human creature you control
/// enters with an additional +1/+1 counter on it.
pub struct DearlyDeparted;

impl CardBehavior for DearlyDeparted {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Dearly Departed".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Flying\nAs long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "Human enters with +1/+1 counter (Dearly Departed in graveyard)".into(),
                },
            ],
        }
    }

    fn trigger_zones(&self, kind: &TriggerKind) -> Vec<Zone> {
        match kind {
            TriggerKind::AnyCreatureEnters => vec![Zone::Graveyard],
            _ => vec![Zone::Battlefield],
        }
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, registry: &CardRegistry) {
        // Dearly Departed must be in OUR graveyard.
        let self_obj = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Graveyard => o,
            _ => return,
        };
        let owner = self_obj.owner;
        // Only applies to creatures we control.
        if entered_controller != owner {
            return;
        }
        // Check if the entered creature is a Human.
        let card_id = state.get_object(entered_id).map(|o| o.card_id);
        let is_human = card_id
            .and_then(|cid| registry.card_data(cid))
            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
            .unwrap_or(false)
            || state.get_object(entered_id)
                .map(|o| o.subtypes.iter().any(|s| s == "Human"))
                .unwrap_or(false);
        if is_human {
            state.add_counters(entered_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

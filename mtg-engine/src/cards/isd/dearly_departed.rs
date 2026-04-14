use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone, CounterType};

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
            triggered_abilities: vec![],
        }
    }

    fn entering_modifier_zones(&self) -> Vec<Zone> {
        vec![Zone::Graveyard]
    }

    fn modify_creature_entering_counters(
        &self,
        state: &GameState,
        self_id: ObjectId,
        entering_id: ObjectId,
        entering_controller: PlayerId,
        registry: &CardRegistry,
    ) -> Vec<(CounterType, u32)> {
        // Must be in our graveyard.
        let owner = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Graveyard => o.owner,
            _ => return vec![],
        };
        if entering_controller != owner {
            return vec![];
        }
        // Only affects Human creatures.
        let is_human = state.get_object(entering_id)
            .is_some_and(|o| {
                let registry_has = registry.card_data(o.card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Human"));
                let instance_has = o.subtypes.iter().any(|s| s == "Human");
                registry_has || instance_has
            });
        if !is_human {
            return vec![];
        }
        vec![(CounterType::PlusOnePlusOne, 1)]
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Village Cannibals — {2}{B} 2/2 Human.
/// Whenever another Human creature dies, put a +1/+1 counter on Village Cannibals.
pub struct VillageCannibals;

impl CardBehavior for VillageCannibals {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Village Cannibals".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever another Human creature dies, put a +1/+1 counter on Village Cannibals.".into(),
            keywords: vec![],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, dead_id: ObjectId, _dead_controller: PlayerId, registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            return;
        }
        let is_human = state.get_object(dead_id)
            .and_then(|o| registry.card_data(o.card_id))
            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
            .unwrap_or(false);
        if is_human {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

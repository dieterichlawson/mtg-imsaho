use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone, CounterType};

/// Somberwald Spider — 2/4 for {4}{G}. Reach.
/// Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
pub struct SomberwaldSpider;

impl CardBehavior for SomberwaldSpider {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Somberwald Spider".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spider".into()],
            power: Some(2),
            toughness: Some(4),
            oracle_text: "Reach (This creature can block creatures with flying.)\nMorbid — This creature enters with two +1/+1 counters on it if a creature died this turn.".into(),
            keywords: vec![Keyword::Reach],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn entering_with_counters(&self, state: &GameState, _self_id: ObjectId, _from_zone: Option<Zone>, _registry: &CardRegistry) -> Vec<(CounterType, u32)> {
        if state.creature_died_this_turn {
            vec![(CounterType::PlusOnePlusOne, 2)]
        } else {
            vec![]
        }
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Somberwald Spider — 2/4 for {4}{G}. Reach.
/// Morbid — When Somberwald Spider enters the battlefield, if a creature died this turn,
/// put two +1/+1 counters on Somberwald Spider.
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
            oracle_text: "Reach\nMorbid — When Somberwald Spider enters the battlefield, if a creature died this turn, put two +1/+1 counters on Somberwald Spider.".into(),
            keywords: vec![Keyword::Reach],
            flashback_cost: None,
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if state.creature_died_this_turn {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, 2);
            state.log(crate::state::LogLevel::Event,
                "Somberwald Spider enters with morbid — two +1/+1 counters".to_string());
        }
    }
}

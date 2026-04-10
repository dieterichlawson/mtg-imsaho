use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, put two +1/+1 counters on it".into(),
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if state.creature_died_this_turn {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, 2);
            state.log(crate::state::LogLevel::Event,
                "Somberwald Spider enters with morbid — two +1/+1 counters".to_string());
        }
    }
}

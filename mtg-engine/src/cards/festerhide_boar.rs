use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Festerhide Boar — 3/3 for {3}{G}. Trample.
/// Morbid — When Festerhide Boar enters the battlefield, if a creature died this turn,
/// put two +1/+1 counters on Festerhide Boar.
pub struct FesterhideBoar;

impl CardBehavior for FesterhideBoar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Festerhide Boar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Boar".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Trample\nMorbid — When Festerhide Boar enters the battlefield, if a creature died this turn, put two +1/+1 counters on Festerhide Boar.".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, put two +1/+1 counters on it".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if state.creature_died_this_turn {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, 2);
            state.log(crate::state::LogLevel::Event,
                "Festerhide Boar enters with morbid — two +1/+1 counters".to_string());
        }
    }
}

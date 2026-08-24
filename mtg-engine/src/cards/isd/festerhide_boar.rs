use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, CounterType};

/// Festerhide Boar — 3/3 for {3}{G}. Trample.
/// Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
///
/// "Enters with" is a replacement effect (CR 614.1c) — counters are on the creature
/// as it enters, not added by a triggered ability after entry.
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
            subtypes: vec!["Boar".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Trample\nMorbid — This creature enters with two +1/+1 counters on it if a creature died this turn.".into(),
            keywords: vec![Keyword::Trample],
            ..Default::default()
        }
    }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        _registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        crate::cards::helpers::enters_with_counters(self_id, event, || {
            if state.creature_died_this_turn {
                vec![(CounterType::PlusOnePlusOne, 2)]
            } else {
                vec![]
            }
        })
    }
}

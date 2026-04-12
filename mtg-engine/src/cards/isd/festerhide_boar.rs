use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Festerhide Boar — 3/3 for {3}{G}. Trample.
/// Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
///
/// "Enters with" is a replacement effect (CR 614.1c) — counters are on the creature
/// as it enters, not added by a triggered ability after entry.
pub struct FesterhideBoar;

impl FesterhideBoar {
    fn apply_morbid_counters(state: &mut GameState, object_id: ObjectId) {
        if !state.creature_died_this_turn {
            return;
        }
        // Guard against double-counting (cast path applies in on_resolve,
        // then on_enter_battlefield fires again).
        let already_has = state.get_object(object_id)
            .map(|o| *o.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0) > 0)
            .unwrap_or(false);
        if already_has {
            return;
        }
        state.add_counters(object_id, CounterType::PlusOnePlusOne, 2);
        state.log(crate::state::LogLevel::Event,
            "Festerhide Boar enters with morbid — two +1/+1 counters".to_string());
    }
}

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
            oracle_text: "Trample\nMorbid — This creature enters with two +1/+1 counters on it if a creature died this turn.".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "enters with +1/+1 counters (morbid)".into(),
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        Self::apply_morbid_counters(state, object_id);
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        // Apply morbid counters immediately so they're visible before ETB triggers.
        Self::apply_morbid_counters(state, object_id);
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Champion of the Parish — {W} 1/1 Human Soldier.
/// Whenever another Human creature enters the battlefield under your control,
/// put a +1/+1 counter on Champion of the Parish.
pub struct ChampionOfTheParish;

impl CardBehavior for ChampionOfTheParish {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Champion of the Parish".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Whenever another Human creature enters the battlefield under your control, put a +1/+1 counter on Champion of the Parish.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "put a +1/+1 counter on Champion of the Parish".into(),
                },
            ],
        }
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Must be under our control
        if entered_controller != controller {
            return;
        }
        // Check if the entered creature is a Human (registry data or instance subtypes)
        let card_id = state.get_object(entered_id).map(|o| o.card_id);
        let is_human = card_id
            .and_then(|cid| registry.card_data(cid))
            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
            .unwrap_or(false)
            || state.get_object(entered_id)
                .map(|o| o.subtypes.iter().any(|s| s == "Human"))
                .unwrap_or(false);
        if is_human {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

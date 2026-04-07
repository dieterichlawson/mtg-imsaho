use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Lumberknot — {2}{G}{G} 1/1 Treefolk. Hexproof.
/// Whenever a creature dies, put a +1/+1 counter on Lumberknot.
pub struct Lumberknot;

impl CardBehavior for Lumberknot {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Lumberknot".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Treefolk".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Hexproof (This creature can't be the target of spells or abilities your opponents control.)\nWhenever a creature dies, put a +1/+1 counter on this creature.".into(),
            keywords: vec![Keyword::Hexproof],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a +1/+1 counter on Lumberknot".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

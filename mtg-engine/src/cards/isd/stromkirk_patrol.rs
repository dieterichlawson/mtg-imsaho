use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Stromkirk Patrol — {4}{B} 4/3 Vampire.
/// Whenever Stromkirk Patrol deals combat damage to a player, put a +1/+1 counter on it.
pub struct StromkirkPatrol;

impl CardBehavior for StromkirkPatrol {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stromkirk Patrol".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into(), "Soldier".into()],
            power: Some(4),
            toughness: Some(3),
            oracle_text: "Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put a +1/+1 counter on Stromkirk Patrol".into(),
                },
            ],
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        if state.get_object(self_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

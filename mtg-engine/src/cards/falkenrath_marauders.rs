use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Falkenrath Marauders — {3}{R}{R} 2/2 Vampire Warrior with flying and haste.
/// Whenever Falkenrath Marauders deals combat damage to a player, put two +1/+1 counters on it.
pub struct FalkenrathMarauders;

impl CardBehavior for FalkenrathMarauders {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Falkenrath Marauders".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into(), "Warrior".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying, haste\nWhenever Falkenrath Marauders deals combat damage to a player, put two +1/+1 counters on Falkenrath Marauders.".into(),
            keywords: vec![Keyword::Flying, Keyword::Haste],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put two +1/+1 counters on Falkenrath Marauders".into(),
                },
            ],
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 2);
        }
    }
}

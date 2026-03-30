use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Bloodcrazed Neonate — {1}{R} 2/1 Vampire.
/// Bloodcrazed Neonate attacks each combat if able.
/// Whenever Bloodcrazed Neonate deals combat damage to a player, put a +1/+1 counter on it.
pub struct BloodcrazedNeonate;

impl CardBehavior for BloodcrazedNeonate {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bloodcrazed Neonate".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Bloodcrazed Neonate attacks each combat if able.\nWhenever Bloodcrazed Neonate deals combat damage to a player, put a +1/+1 counter on Bloodcrazed Neonate.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put a +1/+1 counter on Bloodcrazed Neonate".into(),
                },
            ],
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

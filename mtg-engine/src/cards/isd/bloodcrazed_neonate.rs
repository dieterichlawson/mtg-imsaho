use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope, CounterType};

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
            subtypes: vec!["Vampire".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "This creature attacks each combat if able.\nWhenever this creature deals combat damage to a player, put a +1/+1 counter on it.".into(),
            continuous_effects: vec![
                ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put a +1/+1 counter on Bloodcrazed Neonate".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // `add_counters` is where CR 121.1 says a Neonate that has left the
        // battlefield is not there to take one.
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }
}

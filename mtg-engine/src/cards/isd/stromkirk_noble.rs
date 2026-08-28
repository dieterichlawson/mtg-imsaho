use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, CreatureFilter, EffectScope, CounterType};

/// Stromkirk Noble — {R} 1/1 Vampire Noble.
/// Stromkirk Noble can't be blocked by Humans.
/// Whenever Stromkirk Noble deals combat damage to a player, put a +1/+1 counter on it.
pub struct StromkirkNoble;

impl CardBehavior for StromkirkNoble {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stromkirk Noble".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into(), "Noble".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "This creature can't be blocked by Humans.\nWhenever this creature deals combat damage to a player, put a +1/+1 counter on it.".into(),
            continuous_effects: vec![
                ContinuousEffect::CanOnlyBeBlockedBy {
                    allowed_blockers: CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
                    scope: EffectScope::OnSelf,
                },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put a +1/+1 counter on Stromkirk Noble".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // `add_counters` is where CR 121.1 says a Noble that has left the
        // battlefield is not there to take a counter — for every card at once,
        // rather than here again.
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }
}

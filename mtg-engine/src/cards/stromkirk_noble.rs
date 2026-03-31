use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Vampire".into(), "Noble".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Stromkirk Noble can't be blocked by Humans.\nWhenever Stromkirk Noble deals combat damage to a player, put a +1/+1 counter on Stromkirk Noble.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::BlockRestriction {
                    allowed_blockers: CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
                    scope: EffectScope::OnSelf,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put a +1/+1 counter on Stromkirk Noble".into(),
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

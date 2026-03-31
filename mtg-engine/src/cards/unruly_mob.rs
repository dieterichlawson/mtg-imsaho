use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Unruly Mob — {1}{W} 1/1 Human.
/// Whenever another creature you control dies, put a +1/+1 counter on Unruly Mob.
pub struct UnrulyMob;

impl CardBehavior for UnrulyMob {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Unruly Mob".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Whenever another creature you control dies, put a +1/+1 counter on Unruly Mob.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a +1/+1 counter on Unruly Mob".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if dead_controller == controller {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

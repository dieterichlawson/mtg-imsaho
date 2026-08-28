use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType};
use crate::actions::Target;

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
            subtypes: vec!["Human".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Whenever another creature you control dies, put a +1/+1 counter on this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a +1/+1 counter on Unruly Mob".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    /// "another creature **you control** dies" — a condition on the event, so
    /// it is read as the creature dies (CR 603.2). "Another" is the
    /// collector's: a permanent never sees its own death in the watcher scan.
    fn should_trigger_on_creature_dies(&self, state: &GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _registry: &CardRegistry) -> bool {
        dead_controller == crate::cards::helpers::controller_of(state, self_id)
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // Ruling: a Mob that died alongside the creature "won't be on the
        // battlefield as its triggered ability resolves. It can't be saved by
        // the +1/+1 counter that would have been put on it." That is CR 121.1,
        // and `add_counters` is where it lives.
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }
}

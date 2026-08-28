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

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // The counter goes on the Mob itself, so a Mob that is gone has
        // nothing for this trigger to do.
        if !crate::cards::helpers::still_on_battlefield(state, self_id) {
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, self_id);
        if dead_controller == controller {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

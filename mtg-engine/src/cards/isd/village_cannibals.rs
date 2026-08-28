use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType};
use crate::actions::Target;

/// Village Cannibals — {2}{B} 2/2 Human.
/// Whenever another Human creature dies, put a +1/+1 counter on this creature.
pub struct VillageCannibals;

impl CardBehavior for VillageCannibals {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Village Cannibals".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever another Human creature dies, put a +1/+1 counter on this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a +1/+1 counter on Village Cannibals".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    /// "another **Human** creature dies" — a condition on the event (CR 603.2),
    /// so a Zombie dying is not this ability's event and puts nothing on the
    /// stack.
    ///
    /// Read from the death event, not from the object. By the time anything
    /// asks, the object has been through `move_object`, which clears
    /// `is_transformed` (CR 400.7) — so a werewolf that died *as a Werewolf*
    /// reads back as the Human on its front face and fed the Cannibals a
    /// counter it had not earned. A Human token has it worse: SBA 704.5d has
    /// taken it out of `state.objects` entirely, so it would have read as
    /// nothing at all.
    fn should_trigger_on_creature_dies(&self, _state: &GameState, _self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, dead_subtypes: &[String], _registry: &CardRegistry) -> bool {
        dead_subtypes.iter().any(|s| s == "Human")
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }
}

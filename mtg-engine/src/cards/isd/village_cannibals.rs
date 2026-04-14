use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};
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
            supertypes: vec![],
            subtypes: vec!["Human".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever another Human creature dies, put a +1/+1 counter on this creature.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a +1/+1 counter on Village Cannibals".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], registry: &CardRegistry) {
        if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        let is_human = state.get_object(dead_id)
            .is_some_and(|o| {
                let from_registry = registry.card_data(o.card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Human"));
                let from_instance = o.subtypes.iter().any(|s| s == "Human");
                from_registry || from_instance
            });
        if is_human {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

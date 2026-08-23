use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Champion of the Parish — {W} 1/1 Human Soldier.
/// Whenever another Human you control enters, put a +1/+1 counter on this creature.
pub struct ChampionOfTheParish;

impl CardBehavior for ChampionOfTheParish {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Champion of the Parish".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Whenever another Human you control enters, put a +1/+1 counter on this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "put a +1/+1 counter on Champion of the Parish".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Must be under our control
        if entered_controller != controller {
            return;
        }
        // `has_subtype` reads the ACTIVE face; the previous hand-rolled check
        // used `registry.card_data`, which is always the front face, so a
        // transformed werewolf still counted as a Human here.
        if state.has_subtype(entered_id, "Human", registry) {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType};

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
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Whenever another Human you control enters, put a +1/+1 counter on this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "put a +1/+1 counter on Champion of the Parish".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, registry: &CardRegistry) {
        // The counter goes on the Champion, so a Champion that is gone has
        // nothing for this trigger to do. Asked as its own question, not
        // smuggled into the controller read (CR 608.2g).
        if !crate::cards::helpers::still_on_battlefield(state, self_id) {
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, self_id);
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

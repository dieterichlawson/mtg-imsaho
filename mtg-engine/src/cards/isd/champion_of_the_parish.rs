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

    /// "Whenever another **Human you control** enters" — a condition on the
    /// event, so it is read as the creature enters (CR 603.2).
    ///
    /// It used to be read on resolution instead, which was wrong in both
    /// directions. Every creature entering under any player's control put a
    /// Champion trigger on the stack that then did nothing — a stack object
    /// with a priority window around it. And a Human that entered and stopped
    /// being one before the trigger resolved took the counter away with it:
    /// Moonmist is an instant, and "transform all Human creatures" in response
    /// to this trigger is a real line of play.
    ///
    /// "Another" is the collector's, not this card's — a permanent never sees
    /// its own arrival in the ETB-watch scan.
    ///
    /// `has_subtype` reads the ACTIVE face. The check before it used
    /// `registry.card_data`, which is always the front face, so a transformed
    /// werewolf still counted as a Human.
    fn should_trigger_on_creature_enters(&self, state: &GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, registry: &CardRegistry) -> bool {
        let Some(controller) = state.get_object(self_id).map(|o| o.controller) else { return false };
        entered_controller == controller
            && state.has_subtype(entered_id, "Human", registry)
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, _entered_id: ObjectId, _entered_controller: PlayerId, _registry: &CardRegistry) {
        // The whole condition was settled at trigger time. What is left is the
        // counter, and `add_counters` is where CR 121.1 says a Champion that
        // has left the battlefield is not there to take one.
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }
}

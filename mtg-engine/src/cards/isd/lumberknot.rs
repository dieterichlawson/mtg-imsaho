use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, CounterType};
use crate::actions::Target;

/// Lumberknot — {2}{G}{G} 1/1 Treefolk. Hexproof.
/// Whenever a creature dies, put a +1/+1 counter on Lumberknot.
pub struct Lumberknot;

impl CardBehavior for Lumberknot {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Lumberknot".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Treefolk".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Hexproof (This creature can't be the target of spells or abilities your opponents control.)\nWhenever a creature dies, put a +1/+1 counter on this creature.".into(),
            keywords: vec![Keyword::Hexproof],
            // "Whenever **a** creature dies" — not "another", so its own death
            // is one of them. `AnyCreatureDies` is the watcher kind and means
            // "another creature": the death-watch scan skips the creature that
            // died. A card that also wants its own death declares `SelfDies`
            // beside it, as Falkenrath Noble and Selhoff Occultist do for
            // "this creature or another creature dies".
            //
            // Nothing on any board changes for it: the counter would go on a
            // Lumberknot that is already in the graveyard, and CR 121.1 puts it
            // nowhere. What changes is that the ability triggers at all — a
            // stack object, with a priority window around it, that the rules
            // say is there.
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "put a +1/+1 counter on Lumberknot".into(),
                    target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a +1/+1 counter on Lumberknot".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    /// Its own death, which "a creature dies" includes.
    fn on_dies(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], _registry: &CardRegistry) {
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // `add_counters` is where CR 121.1 says a Lumberknot that has left the
        // battlefield is not there to take one.
        state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
    }
}

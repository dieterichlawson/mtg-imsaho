use rand::seq::SliceRandom;

use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Woodland Sleuth — {3}{G} 2/3 Human Scout.
/// Morbid — When this creature enters, if a creature died this turn,
/// return a creature card at random from your graveyard to your hand.
pub struct WoodlandSleuth;

impl CardBehavior for WoodlandSleuth {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Woodland Sleuth".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Scout".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, return a random creature from graveyard to hand".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn should_trigger(&self, state: &GameState, _self_id: ObjectId, kind: &TriggerKind, _registry: &CardRegistry) -> bool {
        helpers::morbid_should_trigger(state, kind)
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        if !state.creature_died_this_turn {
            return;
        }

        // The Sleuth may have left the battlefield by the time the trigger resolves
        // (e.g., it died in response). We still know who controlled it when it
        // entered, so accept the controller from any zone the object is in.
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Find creature cards in graveyard.
        let mut creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map_or(o.power.is_some(), |d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
            })
            .map(|o| o.id)
            .collect();

        if !creatures.is_empty() {
            let mut rng = rand::thread_rng();
            creatures.shuffle(&mut rng);
            let chosen = creatures[0];
            let name = state.get_object(chosen).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(chosen, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Woodland Sleuth (morbid): returned {name} to hand"));
        }
    }
}

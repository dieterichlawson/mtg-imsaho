use rand::seq::SliceRandom;

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Moldgraf Monstrosity {4}{G}{G}{G} 8/8 Insect with Trample.
/// When Moldgraf Monstrosity dies, exile it, then return two creature cards at random
/// from your graveyard to the battlefield.
pub struct MoldgrafMonstrosity;

impl CardBehavior for MoldgrafMonstrosity {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moldgraf Monstrosity".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Insect".into()],
            power: Some(8),
            toughness: Some(8),
            oracle_text: "Trample\nWhen this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "exile, return two random creatures from graveyard".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        // CR 603.10c: "your" means last-known controller, not owner.
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Exile Moldgraf Monstrosity.
        state.move_object(object_id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            "Moldgraf Monstrosity: exiled on death".into());

        // Find creature cards in the graveyard (excluding the Monstrosity itself, which is now exiled).
        let creatures_in_gy: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| o.power.is_some() && o.id != object_id)
            .map(|o| o.id)
            .collect();

        // Return up to 2 random creatures to the battlefield.
        let mut creatures_in_gy = creatures_in_gy;
        let mut rng = rand::thread_rng();
        creatures_in_gy.shuffle(&mut rng);
        let to_return: Vec<ObjectId> = creatures_in_gy.into_iter().take(2).collect();
        for cid in &to_return {
            let name = state.get_object(*cid).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*cid, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(*cid) {
                obj.controller = controller;
            }
            state.log(crate::state::LogLevel::Event,
                format!("Moldgraf Monstrosity: {name} returned to the battlefield"));
        }
    }
}

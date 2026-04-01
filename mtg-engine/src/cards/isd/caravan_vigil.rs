use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Caravan Vigil — {G} Sorcery.
/// Search your library for a basic land card, reveal it, put it into your hand,
/// then shuffle your library. Morbid — You may put that card onto the battlefield
/// instead of into your hand.
pub struct CaravanVigil;

impl CardBehavior for CaravanVigil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Caravan Vigil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Search your library for a basic land card, reveal it, put it into your hand, then shuffle your library. Morbid — You may put that card onto the battlefield instead of into your hand.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Search library for a basic land card.
        let player = state.get_player(controller);
        let basic_land = player.library_order.iter()
            .find(|&&obj_id| {
                registry.card_data(
                    state.get_object(obj_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0))
                )
                .map(|d| {
                    d.card_types.iter().any(|ct| matches!(ct, CardType::Land))
                        && d.supertypes.iter().any(|st| matches!(st, Supertype::Basic))
                })
                .unwrap_or(false)
            })
            .copied();

        if let Some(land_id) = basic_land {
            let name = state.get_object(land_id).map(|o| o.name.clone()).unwrap_or_default();

            // Remove from library order.
            state.get_player_mut(controller).library_order.retain(|&id| id != land_id);

            if state.creature_died_this_turn {
                // Morbid: "You may put that card onto the battlefield instead."
                // Auto-choose battlefield (strictly better in almost all cases).
                state.move_object(land_id, Zone::Battlefield);
                state.log(crate::state::LogLevel::Event,
                    format!("Caravan Vigil (morbid): {} enters the battlefield", name));
            } else {
                state.move_object(land_id, Zone::Hand);
                state.log(crate::state::LogLevel::Event,
                    format!("Caravan Vigil: {} put into hand", name));
            }

            // Shuffle library.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
        } else {
            state.log(crate::state::LogLevel::Event,
                "Caravan Vigil: no basic land found in library".into());
            // Still shuffle (you searched).
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
        }

        state.move_spell_after_resolve(object_id);
    }
}

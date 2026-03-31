use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Mulch — {1}{G} Sorcery.
/// Reveal the top four cards of your library. Put all land cards revealed this way
/// into your hand and the rest into your graveyard.
pub struct Mulch;

impl CardBehavior for Mulch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mulch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Reveal the top four cards.
        let player = state.get_player_mut(controller);
        let count = std::cmp::min(4, player.library_order.len());
        let revealed: Vec<ObjectId> = player.library_order.drain(..count).collect();

        let mut lands = Vec::new();
        let mut non_lands = Vec::new();

        for &card_id in &revealed {
            let is_land = registry.card_data(
                state.get_object(card_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0))
            )
            .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Land)))
            .unwrap_or(false);

            if is_land {
                lands.push(card_id);
            } else {
                non_lands.push(card_id);
            }
        }

        // Log the reveal.
        let all_names: Vec<String> = revealed.iter()
            .filter_map(|id| state.get_object(*id).map(|o| o.name.clone()))
            .collect();
        state.log(crate::state::LogLevel::Event,
            format!("Mulch revealed: {}", all_names.join(", ")));

        // Lands go to hand.
        for &land_id in &lands {
            let name = state.get_object(land_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(land_id, Zone::Hand);
            state.log(crate::state::LogLevel::Event,
                format!("Mulch: {} put into hand", name));
        }

        // Non-lands go to graveyard.
        for &non_land_id in &non_lands {
            state.move_object(non_land_id, Zone::Graveyard);
        }

        state.move_spell_after_resolve(object_id);
    }
}

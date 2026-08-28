use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Reveal the top four cards.
        let player = state.get_player_mut(controller);
        let count = std::cmp::min(4, player.library_order.len());
        let revealed: Vec<ObjectId> = player.library_order.drain(..count).collect();

        let mut lands = Vec::new();
        let mut non_lands = Vec::new();

        for &card_id in &revealed {
            let is_land = state.has_card_type(card_id, CardType::Land, registry);

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
            state.move_object(land_id, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Mulch: {name} put into hand"));
        }

        // Non-lands go to graveyard. Routed through `mill_one` so a creature
        // card among them emits CreatureCardMilled — moving it directly meant
        // Undead Alchemist never saw it.
        for &non_land_id in &non_lands {
            crate::engine::mill_one(state, non_land_id, registry);
        }

    }
}

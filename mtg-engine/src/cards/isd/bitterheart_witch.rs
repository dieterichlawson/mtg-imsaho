use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Bitterheart Witch — {4}{B} 1/2 Human Shaman with Deathtouch.
/// When Bitterheart Witch dies, you may search your library for a Curse card,
/// put it onto the battlefield attached to target player, then shuffle.
///
/// Simplified: auto-finds the first Curse in library, attaches to opponent.
pub struct BitterheartWitch;

impl CardBehavior for BitterheartWitch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bitterheart Witch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Deathtouch\nWhen Bitterheart Witch dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.".into(),
            keywords: vec![Keyword::Deathtouch],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "search library for a Curse card".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(controller);

        // Search library for a Curse card.
        let curse_id = {
            let player = state.get_player(controller);
            player.library_order.iter()
                .find(|&&obj_id| {
                    let card_id = state.get_object(obj_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0));
                    registry.card_data(card_id)
                        .map(|d| d.subtypes.iter().any(|s| s == "Curse"))
                        .unwrap_or(false)
                })
                .copied()
        };

        if let Some(curse_obj_id) = curse_id {
            let name = state.get_object(curse_obj_id).map(|o| o.name.clone()).unwrap_or_default();
            // Remove from library.
            state.get_player_mut(controller).library_order.retain(|&id| id != curse_obj_id);
            // Put on battlefield attached to opponent.
            state.move_object(curse_obj_id, Zone::Battlefield);
            if let Some(obj) = state.get_object_mut(curse_obj_id) {
                obj.attached_to_player = Some(opponent);
                obj.summoning_sick = false;
            }
            state.log(crate::state::LogLevel::Event,
                format!("Bitterheart Witch found {} and attached it to p{}", name, opponent.0));
            // Shuffle library.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
        } else {
            state.log(crate::state::LogLevel::Event,
                "Bitterheart Witch: no Curse found in library".to_string());
            // Still shuffle.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
        }
    }
}

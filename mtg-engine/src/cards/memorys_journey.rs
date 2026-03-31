use rand::seq::SliceRandom;

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Memory's Journey — {1}{U} Instant.
/// Target player shuffles up to three target cards from their graveyard into their library.
/// Flashback {G}.
pub struct MemorysJourney;

impl CardBehavior for MemorysJourney {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Memory's Journey".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target player shuffles up to three target cards from their graveyard into their library.\nFlashback {G}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        // Targets a player. The graveyard card selection is auto-handled.
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Player(target_player)) = targets.first() {
            // Select up to 3 cards from target player's graveyard.
            let graveyard_cards: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, *target_player)
                .iter()
                .map(|o| o.id)
                .take(3)
                .collect();

            for &card_id in &graveyard_cards {
                let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(card_id, Zone::Library);
                state.get_player_mut(*target_player).library_order.push(card_id);
                state.log(crate::state::LogLevel::Event,
                    format!("Memory's Journey: {} shuffled into library", name));
            }

            // Shuffle the library.
            if !graveyard_cards.is_empty() {
                let mut rng = rand::thread_rng();
                state.get_player_mut(*target_player).library_order.shuffle(&mut rng);
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

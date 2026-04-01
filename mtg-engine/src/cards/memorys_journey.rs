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
        // Up to 3 graveyard cards. The "target player" part is simplified —
        // we target graveyard cards from any player's graveyard.
        TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCard))
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(card_id) = target {
                let owner = state.get_object(*card_id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
                let name = state.get_object(*card_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(*card_id, Zone::Library);
                state.get_player_mut(owner).library_order.push(*card_id);
                state.log(crate::state::LogLevel::Event,
                    format!("Memory's Journey: {} shuffled into library", name));
            }
        }

        // Shuffle only the affected player's library (owner of the cards).
        if !targets.is_empty() {
            use rand::seq::SliceRandom;
            use std::collections::HashSet;
            let mut rng = rand::thread_rng();
            let affected_players: HashSet<u8> = targets.iter()
                .filter_map(|t| if let Target::Object(id) = t { state.get_object(*id).map(|o| o.owner.0) } else { None })
                .collect();
            for &pid in &affected_players {
                if let Some(player) = state.players.get_mut(pid as usize) {
                    player.library_order.shuffle(&mut rng);
                }
            }
        }

        state.move_spell_after_resolve(object_id);
    }
}

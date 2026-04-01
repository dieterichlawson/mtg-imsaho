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
        // Oracle: "Target player shuffles up to three target cards from their graveyard."
        // Cards must all come from one player's graveyard.
        // Mode 1: up to 3 cards from caster's graveyard (targeting self).
        // Mode 2: up to 3 cards from opponent's graveyard (targeting opponent).
        TargetRequirement::ModalChoice(vec![
            TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCardOwnedByCaster)),
            TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCardOwnedByOpponent)),
        ])
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        // Determine which player's graveyard the cards come from.
        let target_player = targets.first().and_then(|t| {
            if let Target::Object(id) = t {
                state.get_object(*id).map(|o| o.owner)
            } else {
                None
            }
        });

        for target in targets {
            if let Target::Object(card_id) = target {
                let (name, owner, in_gy) = match state.get_object(*card_id) {
                    Some(o) => (o.name.clone(), o.owner, o.zone == Zone::Graveyard),
                    None => continue,
                };
                if in_gy {
                    state.move_object(*card_id, Zone::Library);
                    state.get_player_mut(owner).library_order.push(*card_id);
                    state.log(crate::state::LogLevel::Event,
                        format!("Memory's Journey: {} shuffled into library", name));
                }
            }
        }

        // Shuffle the targeted player's library.
        if let Some(player_id) = target_player {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(player_id).library_order.shuffle(&mut rng);
            state.log(crate::state::LogLevel::Event,
                format!("Memory's Journey: p{}'s library shuffled", player_id.0));
        }

        state.move_spell_after_resolve(object_id);
    }
}

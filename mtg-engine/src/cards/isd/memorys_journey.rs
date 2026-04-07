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
            oracle_text: "Target player shuffles up to three target cards from their graveyard into their library.\nFlashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        // Oracle: "Target player shuffles up to three target cards from their graveyard into their library."
        // Requires a mandatory player target plus up to 3 graveyard card targets.
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::PlayerOnly),
            Box::new(TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCard))),
        )
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Determine which player's graveyard the cards come from.
        // With the TwoTargets requirement, the first target is Target::Player.
        let target_player = targets.iter().find_map(|t| {
            if let Target::Player(pid) = t {
                Some(*pid)
            } else {
                None
            }
        }).unwrap_or(controller); // If no player target, default to controller

        for target in targets {
            if let Target::Object(card_id) = target {
                let (name, owner, in_gy) = match state.get_object(*card_id) {
                    Some(o) => (o.name.clone(), o.owner, o.zone == Zone::Graveyard),
                    None => continue,
                };
                if in_gy {
                    state.move_object(*card_id, Zone::Library, registry);
                    state.get_player_mut(owner).library_order.push(*card_id);
                    state.log(crate::state::LogLevel::Event,
                        format!("Memory's Journey: {} shuffled into library", name));
                }
            }
        }

        // Shuffle the targeted player's library (even if no cards were shuffled in).
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(target_player).library_order.shuffle(&mut rng);
            state.log(crate::state::LogLevel::Event,
                format!("Memory's Journey: p{}'s library shuffled", target_player.0));
        }

        state.move_spell_after_resolve(object_id, registry);
    }
}

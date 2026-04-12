use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Traveler's Amulet — {1} Artifact.
/// {1}, Sacrifice this artifact: Search your library for a basic land card,
/// reveal it, put it into your hand, then shuffle.
pub struct TravelersAmulet;

impl CardBehavior for TravelersAmulet {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Traveler's Amulet".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}, Sacrifice: Search library for a basic land, put it into your hand".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::SacrificeThis,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // The artifact was already sacrificed by the engine.
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        // Search library for all basic land cards.
        let basic_lands: Vec<crate::ids::ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&lib_id| {
                state.get_object(lib_id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .map(|d| {
                        d.card_types.contains(&CardType::Land)
                            && d.supertypes.contains(&Supertype::Basic)
                    })
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        if basic_lands.is_empty() {
            state.log(crate::state::LogLevel::Event,
                format!("Traveler's Amulet: p{} found no basic land", controller.0));
            // Still shuffle (you searched).
            use rand::seq::SliceRandom;
            state.get_player_mut(controller).library_order.shuffle(&mut rand::thread_rng());
        } else if basic_lands.len() == 1 {
            let land_id = basic_lands[0];
            state.get_player_mut(controller).library_order.retain(|&id| id != land_id);
            state.move_object(land_id, Zone::Hand, registry);
            let name = state.obj_name(land_id);
            state.log(crate::state::LogLevel::Event,
                format!("Traveler's Amulet: p{} searched for {}", controller.0, name));
            use rand::seq::SliceRandom;
            state.get_player_mut(controller).library_order.shuffle(&mut rand::thread_rng());
        } else {
            // Multiple basic lands — player chooses.
            state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: crate::state::ResolutionChoiceKind::ChooseFromLibrary {
                    description: "Traveler's Amulet: choose a basic land card".into(),
                    options: basic_lands,
                    searcher: controller,
                    source_id: object_id,
                },
            });
            // ChooseFromLibrary handler moves to hand and shuffles.
        }
    }
}

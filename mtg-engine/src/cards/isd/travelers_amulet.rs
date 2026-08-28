use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Zone, Supertype};

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
            oracle_text: "{1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
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
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {

        // The artifact was already sacrificed by the engine.
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        // "Search your library for a basic land card, reveal it, put it into
        // your hand, then shuffle." The search shape itself is general.
        let basic_lands: Vec<crate::ids::ObjectId> = state.get_player(controller).library_order.iter()
            .copied()
            .filter(|&id| is_basic_land(state, id, registry))
            .collect();

        crate::cards::helpers::search_library(
            state, object_id, controller, basic_lands,
            Zone::Hand, false, false,
            "Traveler's Amulet: choose a basic land card",
        );
    }
}

/// A basic land card — `Land` card type plus the `Basic` supertype.
fn is_basic_land(state: &GameState, id: crate::ids::ObjectId, registry: &CardRegistry) -> bool {
    state.has_card_type(id, CardType::Land, registry)
        && state.face_data(id, registry)
            .is_some_and(|d| d.supertypes.contains(&Supertype::Basic))
}

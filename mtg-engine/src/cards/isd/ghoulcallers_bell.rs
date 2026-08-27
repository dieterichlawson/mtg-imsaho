use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Zone};

/// Ghoulcaller's Bell — {1} Artifact.
/// {T}: Each player mills a card.
pub struct GhoulcallersBell;

impl CardBehavior for GhoulcallersBell {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghoulcaller's Bell".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
            ])),
            card_types: vec![CardType::Artifact],
            oracle_text: "{T}: Each player mills a card.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{T}: Each player mills a card".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let player_ids: Vec<crate::ids::PlayerId> = state.players.iter().map(|p| p.id).collect();
        for pid in player_ids {
            crate::engine::mill_cards(state, pid, 1, registry);
        }
    }
}

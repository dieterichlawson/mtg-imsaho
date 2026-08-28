use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType};

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

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // No zone-or-tapped guard here: `legal_actions` enumerates only
        // battlefield permanents its player controls and rejects a
        // `requires_tap` ability on a tapped one, and it also applies the
        // summoning-sickness rule this never did (CR 302.6 — irrelevant to a
        // land, but a card should not be the place that decides).
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
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let player_ids: Vec<crate::ids::PlayerId> = state.players.iter().map(|p| p.id).collect();
        for pid in player_ids {
            crate::engine::mill_cards(state, pid, 1, "Ghoulcaller's Bell", registry);
        }
    }
}

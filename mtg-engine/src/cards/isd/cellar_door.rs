use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Cellar Door — {2} Artifact.
/// {3}, {T}: Target player puts the bottom card of their library into their
/// graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
pub struct CellarDoor;

impl CardBehavior for CellarDoor {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Cellar Door".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{3}, {T}: Target player mills a card, maybe create Zombie".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(3),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::PlayerOnly),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };
        if let Some(Target::Player(player_id)) = targets.first() {
            // Mill the BOTTOM card of the library.
            let player = state.get_player(*player_id);
            if player.library_order.is_empty() {
                return;
            }
            let last_idx = player.library_order.len() - 1;
            let milled_id = player.library_order[last_idx];
            state.get_player_mut(*player_id).library_order.remove(last_idx);
            state.move_object(milled_id, Zone::Graveyard);

            // Check if it was a creature.
            let is_creature = state.get_object(milled_id)
                .map(|o| {
                    registry.card_data(o.card_id)
                        .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                        .unwrap_or(o.power.is_some())
                })
                .unwrap_or(false);

            if is_creature {
                state.create_token_with_subtypes(
                    "Zombie", controller, 2, 2,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![],
                    vec!["Zombie".into()],
                );
                state.log(crate::state::LogLevel::Event,
                    "Cellar Door milled a creature, created a 2/2 Zombie token".to_string());
            } else {
                state.log(crate::state::LogLevel::Event,
                    "Cellar Door milled a non-creature card".to_string());
            }
        }
    }
}

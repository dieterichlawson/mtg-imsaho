use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Zone, Color};

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
            oracle_text: "{3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
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
                counter_cost: None,
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

            // Cellar Door mills from the BOTTOM, which `mill_cards` cannot
            // express — but it is still a mill, so it goes through `mill_one`
            // and emits CreatureCardMilled. Doing the move by hand meant
            // Undead Alchemist's trigger never fired for it.
            let is_creature = state.is_creature(milled_id, registry);
            crate::engine::mill_one(state, *player_id, milled_id, registry);

            if is_creature {
                state.create_token_with_subtypes(
                    "Zombie", controller, 2, 2,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![],
                    vec!["Zombie".into()],
                    registry,
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

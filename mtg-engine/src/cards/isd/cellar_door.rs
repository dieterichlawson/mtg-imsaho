use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Color};

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

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // No zone-or-tapped guard here: `legal_actions` enumerates only
        // battlefield permanents its player controls and rejects a
        // `requires_tap` ability on a tapped one, and it also applies the
        // summoning-sickness rule this never did (CR 302.6 — irrelevant to a
        // land, but a card should not be the place that decides).
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
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        // CR 602.2a: an activated ability's controller is the player who
        // activated it, which the engine records; CR 608.2g falls back to the
        // source's last known controller. Reading `o.controller` here gave the
        // *current* controller, so an opponent taking the permanent in
        // response to the ability collected the effect — and `None => return`
        // threw the whole effect away if the source had left, against
        // CR 113.7a.
        let controller = crate::cards::helpers::ability_controller(state, object_id);
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
                    "", controller, 2, 2,
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

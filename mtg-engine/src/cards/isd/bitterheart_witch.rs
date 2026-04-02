use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Bitterheart Witch — {4}{B} 1/2 Human Shaman with Deathtouch.
/// When Bitterheart Witch dies, you may search your library for a Curse card,
/// put it onto the battlefield attached to target player, then shuffle.
pub struct BitterheartWitch;

impl CardBehavior for BitterheartWitch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bitterheart Witch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Deathtouch\nWhen Bitterheart Witch dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.".into(),
            keywords: vec![Keyword::Deathtouch],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "search library for a Curse card".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));

        // "you may" — present a yes/no choice before searching.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: object_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Bitterheart Witch: search your library for a Curse card?".into(),
                source_card: object_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            return;
        }

        let controller = state.get_object(self_id).map(|o| o.controller).unwrap_or(PlayerId(0));

        // Search library for Curse cards.
        let curse_ids: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&obj_id| {
                let card_id = state.get_object(obj_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0));
                registry.card_data(card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Curse"))
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        if curse_ids.is_empty() {
            state.log(LogLevel::Event,
                "Bitterheart Witch: no Curse found in library".to_string());
            // Still shuffle.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
            return;
        }

        // Pick the first matching Curse (single-card tutor).
        let chosen_curse = curse_ids[0];

        // Present a player choice for "target player" to attach the Curse to.
        let player_targets: Vec<crate::actions::Target> = (0..state.players.len())
            .map(|i| crate::actions::Target::Player(PlayerId(i as u8)))
            .collect();

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Bitterheart Witch: choose a player to attach the Curse to".into(),
                options: player_targets,
                optional: false,
                effect: PendingEffect::AttachCurseToPlayer {
                    curse_id: chosen_curse,
                    searcher: controller,
                },
            },
        });
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Bitterheart Witch — {4}{B} 1/2 Human Shaman with Deathtouch.
/// When Bitterheart Witch dies, you may search your library for a Curse card,
/// put it onto the battlefield attached to target player, then shuffle.
pub struct BitterheartWitch;

impl BitterheartWitch {
    /// Present the "target player" choice after a Curse has been selected.
    fn present_player_choice(state: &mut GameState, self_id: ObjectId, controller: PlayerId, curse_id: ObjectId, registry: &CardRegistry) {
        let player_targets: Vec<crate::actions::Target> = (0..state.players.len())
            .map(|i| PlayerId(u8::try_from(i).unwrap_or(u8::MAX)))
            .filter(|&pid| !state.player_has_hexproof(pid, registry) || pid == controller)
            .map(crate::actions::Target::Player)
            .collect();

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Bitterheart Witch: choose a player to attach the Curse to".into(),
                options: player_targets,
                optional: false,
                effect: PendingEffect::AttachCurseToPlayer {
                    curse_id,
                    searcher: controller,
                },
            },
        });
    }
}

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
            oracle_text: "Deathtouch\nWhen this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.".into(),
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
        let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);

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
        use rand::seq::SliceRandom;

        if !yes {
            return;
        }

        let controller = state.get_object(self_id).map_or(PlayerId(0), |o| o.controller);

        // Search library for Curse cards.
        let curse_ids: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&obj_id| {
                let card_id = state.get_object(obj_id).map_or(crate::ids::CardId(0), |o| o.card_id);
                registry.card_data(card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Curse"))
            })
            .copied()
            .collect();

        if curse_ids.is_empty() {
            state.log(LogLevel::Event,
                "Bitterheart Witch: no Curse found in library".to_string());
            // Still shuffle.
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
            return;
        }

        if curse_ids.len() == 1 {
            // Only one Curse — auto-select it, then choose target player.
            let chosen_curse = curse_ids[0];
            Self::present_player_choice(state, self_id, controller, chosen_curse, registry);
        } else {
            // Multiple Curses — player chooses which one via ChooseTarget.
            let curse_targets: Vec<crate::actions::Target> = curse_ids.iter()
                .map(|&id| crate::actions::Target::Object(id))
                .collect();
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Bitterheart Witch: choose a Curse card from your library".into(),
                    options: curse_targets,
                    optional: false,
                    effect: PendingEffect::ChooseCurseThenAttach {
                        searcher: controller,
                        source: self_id,
                    },
                },
            });
        }
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};
use crate::actions::Target;

/// Bitterheart Witch — {4}{B} 1/2 Human Shaman with Deathtouch.
/// When Bitterheart Witch dies, you may search your library for a Curse card,
/// put it onto the battlefield attached to target player, then shuffle.
pub struct BitterheartWitch;

impl BitterheartWitch {
    /// Present the "target player" choice after a Curse has been selected.
    fn present_player_choice(state: &mut GameState, self_id: ObjectId, controller: PlayerId, curse_id: ObjectId, registry: &CardRegistry) {
        // Two separate restrictions. Hexproof stops the ability targeting the
        // player at all; protection from the Curse's color stops the Curse
        // being attached to them even if they could be targeted (CR 702.16b),
        // which makes them an illegal choice for "attached to target player".
        let player_targets: Vec<crate::actions::Target> = (0..state.players.len())
            .map(|i| PlayerId(u8::try_from(i).unwrap_or(u8::MAX)))
            .filter(|&pid| !state.player_has_hexproof(pid, registry) || pid == controller)
            .filter(|&pid| state.player_can_be_enchanted_by(curse_id, pid, registry))
            .map(crate::actions::Target::Player)
            .collect();

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Bitterheart Witch: choose a player to attach the Curse to".into(),
                options: player_targets,
                optional: false,
                effect: PendingEffect::CardEffect {
                    source_id: self_id,
                    // Step 2: the Curse is chosen, the player is not yet.
                    key: format!("attach:{}", curse_id.0),
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
                target_requirement: None,
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], _registry: &CardRegistry) {
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
                state.has_subtype(obj_id, "Curse", registry)
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
                    effect: PendingEffect::CardEffect {
                        source_id: self_id,
                        key: "choose".into(),
                    },
                },
            });
        }
    }

    /// "When this creature dies, you may search your library for a Curse card,
    /// put it onto the battlefield attached to target player, then shuffle."
    /// Two chained choices — which Curse, then which player — so `key` names
    /// which step this is. The engine only routes the answer back here.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, key: &str, target: &Target, registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, source_id);

        if key == "choose" {
            // Step 1 answered: this is the Curse. Now ask for the player.
            let Target::Object(curse_id) = target else { return };
            Self::present_player_choice(state, source_id, controller, *curse_id, registry);
            return;
        }

        // Step 2 answered: attach the chosen Curse to the chosen player.
        let Some(curse_id) = key.strip_prefix("attach:")
            .and_then(|n| n.parse().ok())
            .map(ObjectId) else { return };
        let Target::Player(pid) = target else { return };

        let name = state.obj_name(curse_id);
        // CR 303.4h: an Aura that would enter attached to something it can't
        // legally enchant doesn't enter the battlefield — it stays where it
        // is. The shuffle below still happens; the search did.
        if !state.player_can_be_enchanted_by(curse_id, *pid, registry) {
            state.log(crate::state::LogLevel::Event,
                format!("Bitterheart Witch: {name} can't enchant p{} and stays in the library", pid.0));
        } else {
            state.get_player_mut(controller).library_order.retain(|&id| id != curse_id);
            state.move_object(curse_id, crate::types::Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(curse_id) {
                obj.attached_to_player = Some(*pid);
                obj.summoning_sick = false;
            }
            state.log(crate::state::LogLevel::Event,
                format!("Bitterheart Witch: attached {name} to p{}", pid.0));
        }

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        state.get_player_mut(controller).library_order.shuffle(&mut rng);
    }
}

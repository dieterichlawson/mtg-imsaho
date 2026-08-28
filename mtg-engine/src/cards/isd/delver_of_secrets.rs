use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{CardType, ManaCost, ManaSymbol, Color, Keyword};
use crate::actions::Target;

/// Delver of Secrets {U} 1/1 Human Wizard // Insectile Aberration 3/2 Human Insect with Flying.
/// At the beginning of your upkeep, look at the top card of your library. You may reveal that
/// card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
pub struct DelverOfSecrets;

impl DelverOfSecrets {
    /// Check if the top card of the given player's library is an instant or sorcery.
    fn top_card_is_instant_or_sorcery(state: &GameState, controller: crate::ids::PlayerId, registry: &CardRegistry) -> bool {
        let top_card_id = state.get_player(controller).library_order.first().copied();
        if let Some(top_id) = top_card_id {
            state.has_card_type(top_id, CardType::Instant, registry)
                || state.has_card_type(top_id, CardType::Sorcery, registry)
        } else {
            false
        }
    }
}

impl CardBehavior for DelverOfSecrets {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Delver of Secrets".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "look at top card, may reveal to transform".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Insectile Aberration".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Insect".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "Flying".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        })
    }


    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        // The trigger transforms the Delver, so a Delver that is gone has
        // nothing for it to do.
        if !crate::cards::helpers::still_on_battlefield(state, self_id) {
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let is_transformed = state.get_object(self_id).is_some_and(|o| o.is_transformed);
        // Only trigger on the front face, during controller's upkeep.
        if is_transformed {
            return;
        }
        // "look at the top card of your library. You may reveal that card." —
        // with an empty library there is no card to look at and nothing to
        // reveal, so there is no choice to offer. CR 608.2: the ability does as
        // much as it can, which here is nothing. This used to prompt "reveal
        // nothing from the top of your library?", a decision with no meaning
        // behind it.
        let Some(top_card_id) = state.get_player(controller).library_order.first().copied() else {
            state.log(LogLevel::Debug,
                "Delver of Secrets: library is empty, nothing to look at".into());
            return;
        };
        let top_is_instant_or_sorcery = Self::top_card_is_instant_or_sorcery(state, controller, registry);

        // Log what was seen. Debug level on purpose: the controller looks at
        // this card, the opponent does not, and only `display_log` (Info and
        // above) is shown to players.
        let top_card_name = state.obj_name(top_card_id);
        state.log(LogLevel::Debug,
            format!("Delver of Secrets: top card is {top_card_name}"));

        // Always present the "you may reveal" choice. Per the ruling, the player may reveal
        // any top card. Only if the revealed card is an instant or sorcery does Delver transform.
        let description = if top_is_instant_or_sorcery {
            format!(
                "Delver of Secrets: reveal {top_card_name} from the top of your library to transform?"
            )
        } else {
            format!(
                "Delver of Secrets: reveal {top_card_name} from the top of your library? (not an instant or sorcery — no transform)"
            )
        };
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description,
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            // Player chose not to reveal — card stays on top, no transform.
            state.log(LogLevel::Event, "Delver of Secrets: chose not to reveal".into());
            return;
        }
        // Player reveals the top card. Only transform if it's an instant or sorcery.
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        let top_card_id = state.get_player(controller).library_order.first().copied();
        let top_card_name = top_card_id.map_or_else(|| "a card".into(), |id| state.obj_name(id));
        let top_is_instant_or_sorcery = Self::top_card_is_instant_or_sorcery(state, controller, registry);
        if top_is_instant_or_sorcery {
            state.log(LogLevel::Event,
                format!("Delver of Secrets: reveals {top_card_name}"));
            crate::cards::helpers::apply_transform(state, self_id, registry);
        } else {
            state.log(LogLevel::Event,
                format!("Delver of Secrets: reveals {top_card_name} — not an instant or sorcery, no transform."));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

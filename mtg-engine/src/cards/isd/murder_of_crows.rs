use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::engine::draw_cards;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

/// Murder of Crows — {3}{U}{U} 4/4 Bird. Flying.
/// Whenever another creature dies, you may draw a card. If you do, discard a card.
pub struct MurderOfCrows;

impl CardBehavior for MurderOfCrows {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Murder of Crows".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Bird".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "you may draw a card, then discard a card".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };

        // "You may draw a card. If you do, discard a card."
        // Present yes/no choice. If yes, the engine draws and presents discard.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Murder of Crows: draw a card? (you must then discard one)".into(),
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            return;
        }
        // "You may draw a card. If you do, discard a card."
        let controller = state.get_object(self_id)
            .map_or(PlayerId(0), |o| o.controller);
        draw_cards(state, controller, 1, registry);
        let hand: Vec<_> = state.objects_in_zone(Zone::Hand, controller)
            .iter().map(|o| o.id).collect();
        if hand.len() == 1 {
            state.move_object(hand[0], Zone::Graveyard, registry);
            state.events.push(GameEvent::Discarded { player: controller, object: hand[0] });
            state.log(LogLevel::Event, "Drew and discarded a card".to_string());
        } else if !hand.is_empty() {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseCardFromHand {
                    description: "Murder of Crows: choose a card to discard".into(),
                    player: controller,
                    cards: hand,
                },
            });
        }
    }
}

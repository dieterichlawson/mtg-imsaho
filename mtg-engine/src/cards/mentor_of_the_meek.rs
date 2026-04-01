use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind, YesNoEffect};
use crate::types::*;

/// Mentor of the Meek — {2}{W} 2/2 Human Soldier.
/// Whenever another creature with power 2 or less enters the battlefield
/// under your control, you may pay {1}. If you do, draw a card.
///
/// Simplified: auto-draws if the controller has any mana in pool (pays {1}).
pub struct MentorOfTheMeek;

impl CardBehavior for MentorOfTheMeek {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mentor of the Meek".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever another creature with power 2 or less enters the battlefield under your control, you may pay {1}. If you do, draw a card.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "you may pay {1} to draw a card".into(),
                },
            ],
        }
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, registry: &CardRegistry) {
        // Must be on the battlefield.
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Must be under our control and not self.
        if entered_controller != controller || entered_id == self_id {
            return;
        }
        // Check if the entering creature has power 2 or less.
        let power = state.effective_power(entered_id, registry).unwrap_or(99);
        if power > 2 {
            return;
        }
        // "You may pay {1}. If you do, draw a card." — present choice if mana available.
        let pool = &state.get_player(controller).mana_pool;
        if pool.total() >= 1 {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::YesNo {
                    description: "Mentor of the Meek: pay {1} to draw a card?".into(),
                    source_card: self_id,
                    effect: YesNoEffect::PayAndDraw,
                },
            });
        }
    }
}

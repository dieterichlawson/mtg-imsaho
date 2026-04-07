use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::*;

/// Mentor of the Meek — {2}{W} 2/2 Human Soldier.
/// Whenever another creature with power 2 or less enters the battlefield
/// under your control, you may pay {1}. If you do, draw a card.
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
            oracle_text: "Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.".into(),
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
        // "You may pay {1}" — present choice to the player.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Mentor of the Meek: pay {1} to draw a card?".into(),
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            return;
        }
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Pay {1} (prefer colorless, then any color).
        let player = state.get_player_mut(controller);
        let paid;
        if player.mana_pool.get(ManaType::Colorless) >= 1 {
            *player.mana_pool.mana.entry(ManaType::Colorless).or_insert(0) -= 1;
            paid = true;
        } else {
            let mut found = false;
            for mt in &[ManaType::White, ManaType::Blue, ManaType::Black, ManaType::Red, ManaType::Green] {
                if player.mana_pool.get(*mt) >= 1 {
                    *player.mana_pool.mana.entry(*mt).or_insert(0) -= 1;
                    found = true;
                    break;
                }
            }
            paid = found;
        }
        if paid {
            crate::engine::draw_cards(state, controller, 1, registry);
            state.log(LogLevel::Event, "Mentor of the Meek: paid {1}, drew a card".into());
        }
    }
}

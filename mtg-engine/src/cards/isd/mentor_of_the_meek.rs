use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "you may pay {1} to draw a card".into(),
                    target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    /// "Whenever another creature with power 2 or less enters under your
    /// control..." — an event condition, so it is read as the creature enters.
    fn should_trigger_on_creature_enters(&self, state: &GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: crate::ids::PlayerId, registry: &CardRegistry) -> bool {
        if entered_id == self_id {
            return false; // "another creature"
        }
        let Some(controller) = state.get_object(self_id).map(|o| o.controller) else { return false };
        if entered_controller != controller {
            return false; // "under your control"
        }
        state.effective_power(entered_id, registry).is_some_and(|p| p <= 2)
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, _entered_id: ObjectId, _entered_controller: PlayerId, _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        // The whole condition — another creature you control, power 2 or less —
        // was already checked at dispatch time in `should_trigger_on_creature_
        // enters`. Re-checking power here would be wrong, not just redundant:
        // a creature that entered with power 2 and was pumped in response
        // still triggered, and one that entered with power 3 and shrank did
        // not (CR 603.2).
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

    /// "you may pay {1}. If you do, draw a card."
    ///
    /// Paying may involve tapping lands for the mana (CR 601.2g via 608.2g),
    /// which is what `engine::pay_cost_with_sources` does. This used to walk
    /// the mana pool by hand — colorless first, then WUBRG — and spend a
    /// floating unit if it found one. With an empty pool and four untapped
    /// Plains, saying "yes" quietly paid nothing and drew nothing. Screeching
    /// Bat, the set's other "you may pay", has always gone through the engine.
    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            state.log(LogLevel::Event, "Mentor of the Meek: chose not to pay {1}".into());
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        if !crate::engine::pay_cost_with_sources(state, controller, &cost, registry) {
            state.log(LogLevel::Event, "Mentor of the Meek: could not pay {1}".into());
            return;
        }
        let _ = crate::engine::draw_cards(state, controller, 1, registry);
        state.log(LogLevel::Event, "Mentor of the Meek: paid {1}, drew a card".into());
    }
}

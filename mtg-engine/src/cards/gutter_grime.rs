use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Gutter Grime — {4}{G} Enchantment.
/// Whenever a nontoken creature you control dies, put a slime counter on
/// Gutter Grime, then create a green Ooze creature token with
/// "This creature's power and toughness are each equal to the number of
/// slime counters on Gutter Grime."
///
/// Simplified: tokens are created with P/T equal to the current slime counter
/// count at creation time (they don't dynamically update).
pub struct GutterGrime;

/// Counter type for slime counters — we reuse PlusOnePlusOne as a stand-in
/// since the engine only has a few counter types. The count is tracked on the
/// enchantment itself.
const SLIME_COUNTER: CounterType = CounterType::PlusOnePlusOne;

impl CardBehavior for GutterGrime {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Gutter Grime".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with \"This creature's power and toughness are each equal to the number of slime counters on Gutter Grime.\"".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a slime counter, create Ooze token".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Must be our creature.
        if dead_controller != controller {
            return;
        }
        // Must be a nontoken creature.
        let was_token = state.get_object(dead_id).map(|o| o.is_token).unwrap_or(false);
        if was_token {
            return;
        }
        // Put a slime counter on Gutter Grime.
        state.add_counters(self_id, SLIME_COUNTER, 1);
        // Get the new counter count for the token's P/T.
        let slime_count = state.get_object(self_id)
            .map(|o| *o.counters.get(&SLIME_COUNTER).unwrap_or(&0))
            .unwrap_or(1) as i32;
        // Create the Ooze token.
        state.create_token_with_subtypes(
            "Ooze", controller, slime_count, slime_count,
            vec![Color::Green],
            vec![CardType::Creature],
            vec![],
            vec!["Ooze".into()],
        );
        state.log(crate::state::LogLevel::Event,
            format!("Gutter Grime: added slime counter (now {}), created {}/{} Ooze token",
                slime_count, slime_count, slime_count));
    }
}

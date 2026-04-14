use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Gutter Grime — {4}{G} Enchantment.
/// Whenever a nontoken creature you control dies, put a slime counter on
/// Gutter Grime, then create a green Ooze creature token with
/// "This creature's power and toughness are each equal to the number of
/// slime counters on Gutter Grime."
///
/// Tokens have dynamic P/T that tracks the current slime counter count
/// on the source Gutter Grime enchantment (via `card_state` "`pt_source_counter`").
pub struct GutterGrime;

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
            oracle_text: "Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on Gutter Grime.\"".into(),
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

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Must be our creature.
        if dead_controller != controller {
            return;
        }
        // Must be a nontoken creature.
        let was_token = state.get_object(dead_id).is_some_and(|o| o.is_token);
        if was_token {
            return;
        }
        // Put a slime counter on Gutter Grime.
        state.add_counters(self_id, CounterType::Slime, 1);
        let slime_count = state.get_object(self_id)
            .map_or(1, |o| *o.counters.get(&CounterType::Slime).unwrap_or(&0));
        // Create the Ooze token with base 0/0 and dynamic P/T linked to this Gutter Grime.
        let token_ids = state.create_token_with_subtypes(
            "Ooze", controller, 0, 0,
            vec![Color::Green],
            vec![CardType::Creature],
            vec![],
            vec!["Ooze".into()],
            registry,
        );
        // Link the token's P/T to this Gutter Grime's slime counters.
        // pt_source_counter = ObjectId of the Gutter Grime
        // pt_source_counter_type = 1 means Slime counter type
        for token_id in token_ids {
            if let Some(token) = state.get_object_mut(token_id) {
                token.card_state.insert("pt_source_counter".into(), self_id);
                token.card_state.insert("pt_source_counter_type".into(), ObjectId(1));
            }
        }
        state.log(crate::state::LogLevel::Event,
            format!("Gutter Grime: added slime counter (now {slime_count}), created */* Ooze token (dynamic P/T)"));
    }
}

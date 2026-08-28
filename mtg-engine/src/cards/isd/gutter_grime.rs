use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType};
use crate::actions::Target;

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
            oracle_text: "Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on Gutter Grime.\"".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "put a slime counter, create Ooze token".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, dead_is_token: bool, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        // Must be our creature.
        if dead_controller != controller {
            return;
        }
        // Must be a nontoken creature. Use the captured `dead_is_token` because
        // by the time this trigger resolves, SBA 704.5d has already removed the
        // dead token from `state.objects`, so we can't read `is_token` from the
        // object any more.
        if dead_is_token {
            return;
        }
        // Put a slime counter on Gutter Grime.
        state.add_counters(self_id, CounterType::Slime, 1);
        let slime_count = state.get_counter_count(self_id, CounterType::Slime);
        // Create the Ooze token with base 0/0 and dynamic P/T linked to this Gutter Grime.
        let token_ids = state.create_token_with_subtypes(
            "", controller, 0, 0,
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

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CounterType};
use crate::actions::Target;

/// Gutter Grime — {4}{G} Enchantment.
/// Whenever a nontoken creature you control dies, put a slime counter on
/// Gutter Grime, then create a green Ooze creature token with
/// "This token's power and toughness are each equal to the number of
/// slime counters on Gutter Grime."
///
/// The Ooze token's power and toughness are its own ability, not the Grime's
/// (CR 604.3), so the token records which Gutter Grime made it and this card
/// answers for it through `token_dynamic_pt`. Ruling: "If you control more
/// than one Gutter Grime, each Ooze token remembers which one created it."
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

    /// "Whenever a **nontoken** creature **you control** dies" — both are
    /// conditions on the event (CR 603.2).
    ///
    /// `dead_is_token` comes from the death event rather than the object: SBA
    /// 704.5d has already taken the token out of `state.objects` by the time
    /// anything asks, so its own record of itself is gone (CR 608.2g).
    fn should_trigger_on_creature_dies(&self, state: &GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, dead_is_token: bool, _registry: &CardRegistry) -> bool {
        !dead_is_token && dead_controller == crate::cards::helpers::controller_of(state, self_id)
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
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
        // Ruling: "each Ooze token remembers which one created it" — so the
        // link is to *this* Gutter Grime, not to the card.
        for token_id in token_ids {
            if let Some(token) = state.get_object_mut(token_id) {
                token.card_state.insert(crate::cards::PT_DEFINED_BY.into(), self_id);
            }
        }
        state.log(crate::state::LogLevel::Event,
            format!("Gutter Grime: added slime counter (now {slime_count}), created */* Ooze token (dynamic P/T)"));
    }

    fn token_dynamic_pt(&self, state: &GameState, source_id: ObjectId, _token_id: ObjectId, _registry: &CardRegistry) -> Option<(i32, i32)> {
        // "This token's power and toughness are each equal to the number of
        // slime counters on Gutter Grime." Ruling: "The power and toughness of
        // the Ooze tokens will constantly update as Gutter Grime accumulates
        // slime counters" — so this is read every time, never stamped on the
        // token.
        //
        // `source_id` is the Gutter Grime that made this token. Ruling: "If
        // Gutter Grime leaves the battlefield, the power and toughness of each
        // Ooze token it created will become 0" — which falls out, because a
        // permanent that changes zones loses its counters (CR 400.7).
        let n = i32::try_from(state.get_counter_count(source_id, CounterType::Slime))
            .unwrap_or(i32::MAX);
        Some((n, n))
    }
}

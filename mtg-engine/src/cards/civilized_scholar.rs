use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Civilized Scholar {2}{U} 0/1 Human Advisor // Homicidal Brute 5/1 Human Mutant.
/// {T}: Draw a card, then discard a card. If a creature card is discarded this way,
/// untap Civilized Scholar, then transform Civilized Scholar.
/// Homicidal Brute: At the beginning of your end step, if Homicidal Brute didn't attack
/// this turn, transform Homicidal Brute.
///
/// Simplified: The draw-discard is implemented as an activated ability. If a creature card
/// is in hand, the discard automatically picks a creature (for simplicity). The end-step
/// transform-back checks card_state for whether it attacked.
pub struct CivilizedScholar;

impl CardBehavior for CivilizedScholar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Civilized Scholar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Advisor".into()],
            power: Some(0),
            toughness: Some(1),
            oracle_text: "{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap Civilized Scholar, then transform Civilized Scholar.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "mark as attacked this turn".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "transform back if didn't attack".into(),
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Homicidal Brute".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Mutant".into()],
            power: Some(5),
            toughness: Some(1),
            oracle_text: "At the beginning of your end step, if Homicidal Brute didn't attack this turn, transform Homicidal Brute.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((5, 1))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield && !o.is_transformed => o,
            _ => return vec![],
        };
        if obj.tapped { return vec![]; }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{T}: Draw a card, then discard a card. If creature discarded, untap and transform.".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Draw a card.
        crate::engine::draw_cards(state, controller, 1);

        // Discard a card. Auto-pick a creature if possible (simplified).
        let hand: Vec<_> = state.objects_in_zone(Zone::Hand, controller)
            .iter().map(|o| (o.id, o.power.is_some())).collect();
        if hand.is_empty() {
            return;
        }

        // Prefer to discard a creature card to trigger the transform.
        let (discard_id, is_creature) = hand.iter()
            .find(|(_, is_creature)| *is_creature)
            .or(hand.first())
            .copied()
            .unwrap();

        state.move_object(discard_id, Zone::Graveyard);
        state.events.push(crate::events::GameEvent::Discarded {
            player: controller,
            object: discard_id,
        });
        let discard_name = state.get_object(discard_id).map(|o| o.name.clone()).unwrap_or_default();
        state.log(crate::state::LogLevel::Event,
            format!("Civilized Scholar: p{} discarded {}", controller.0, discard_name));

        if is_creature {
            // Untap and transform.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.tapped = false;
                obj.is_transformed = true;
                obj.name = "Homicidal Brute".into();
            }
            state.log(crate::state::LogLevel::Event,
                "Civilized Scholar transforms into Homicidal Brute".into());
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        // Mark that we attacked this turn (so end-step doesn't transform back).
        if let Some(obj) = state.get_object_mut(self_id) {
            obj.card_state.insert("attacked_this_turn".into(), crate::ids::ObjectId(1));
        }
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        if !is_transformed || state.active_player != controller {
            return;
        }
        // Check if Homicidal Brute attacked this turn.
        let attacked = state.get_object(self_id)
            .map(|o| o.card_state.contains_key("attacked_this_turn"))
            .unwrap_or(false);
        if !attacked {
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.is_transformed = false;
                obj.name = "Civilized Scholar".into();
            }
            state.log(crate::state::LogLevel::Event,
                "Homicidal Brute transforms back into Civilized Scholar (didn't attack)".into());
        }
        // Clear the attack flag for next turn.
        if let Some(obj) = state.get_object_mut(self_id) {
            obj.card_state.remove("attacked_this_turn");
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

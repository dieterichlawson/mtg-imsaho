use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Civilized Scholar {2}{U} 0/1 Human Advisor // Homicidal Brute 5/1 Human Mutant.
/// {T}: Draw a card, then discard a card. If a creature card is discarded this way,
/// untap this creature, then transform it.
/// Homicidal Brute: At the beginning of your end step, if this creature didn't attack
/// this turn, tap this creature, then transform it.
///
/// The draw-discard is an activated ability: after drawing, the player chooses
/// which card to discard, and a discarded creature card untaps and transforms
/// the Scholar. The end-step transform-back asks the engine whether this
/// permanent attacked (`GameState::attacked_this_turn`) — that is a fact about
/// the game, not an ability of this card, and the card used to record it by
/// declaring an `Attacks` trigger on each face whose only job was bookkeeping.
/// Those went on the stack like any other trigger, so Civilized Scholar put a
/// visible, respondable "mark as attacked this turn" ability on the stack every
/// time it attacked — an ability it does not have.
pub struct CivilizedScholar;

impl CivilizedScholar {
    /// "If a creature **card** is discarded this way" — a question about the
    /// card's printed type, which lives on its active face (CR 711.5), not
    /// about `obj.power`. `obj.power` holds runtime grants only for a
    /// registry-backed card, so testing it first meant the authoritative face
    /// data was consulted only when the object field happened to be unset.
    fn is_creature_card(state: &GameState, id: ObjectId, registry: &CardRegistry) -> bool {
        state.face_data(id, registry)
            .is_some_and(|d| d.card_types.contains(&CardType::Creature))
    }

}

impl CardBehavior for CivilizedScholar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Civilized Scholar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Advisor".into()],
            power: Some(0),
            toughness: Some(1),
            oracle_text: "{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.".into(),
            // Civilized Scholar has no triggered ability. Its only ability is
            // the activated one above; the end-step transform-back belongs to
            // Homicidal Brute, on the back face.
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Homicidal Brute".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Mutant".into()],
            power: Some(5),
            toughness: Some(1),
            oracle_text: "At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "transform back if didn't attack".into(),
                    target_requirement: None,
                },
            ],
            ..Default::default()
        })
    }

    fn step_trigger_scope(&self, kind: &TriggerKind, is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::EndStep if is_back_face => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }


    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield && !o.is_transformed => o,
            _ => return vec![],
        };
        let _ = obj;
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{T}: Draw a card, then discard a card. If creature discarded, untap and transform.".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Draw a card.
        let _ = crate::engine::draw_cards(state, controller, 1, registry);

        // Player must choose which card to discard.
        let hand: Vec<_> = state.objects_in_zone(Zone::Hand, controller)
            .iter().map(|o| o.id).collect();
        if hand.is_empty() {
            return;
        }
        if hand.len() == 1 {
            // Only one card — auto-discard and check creature.
            let discard_id = hand[0];
            let is_creature = Self::is_creature_card(state, discard_id, registry);
            state.discard_card(discard_id, registry);
            let discard_name = state.get_object(discard_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("Civilized Scholar: p{} discarded {}", controller.0, discard_name));
            if is_creature {
                // "untap this creature, **then** transform it" — in that order,
                // with no priority in between (ruling, 2011-09-22).
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.tapped = false;
                }
                crate::cards::helpers::apply_transform(state, object_id, registry);
                state.log(crate::state::LogLevel::Event,
                    "Civilized Scholar transforms into Homicidal Brute".into());
            }
        } else {
            // Multiple cards — present choice to player.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseCardFromHand {
                    description: "Civilized Scholar: choose a card to discard".into(),
                    player: controller,
                    cards: hand,
                    discard_immediately: true,
                },
            });
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, discarded_id: ObjectId, registry: &CardRegistry) {
        let is_creature = Self::is_creature_card(state, discarded_id, registry);
        if is_creature {
            // "untap this creature, **then** transform it", in that order.
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.tapped = false;
            }
            crate::cards::helpers::apply_transform(state, self_id, registry);
            state.log(crate::state::LogLevel::Event,
                "Civilized Scholar transforms into Homicidal Brute".into());
        }
    }

    /// CR 603.4: "At the beginning of your end step, **if** this creature
    /// didn't attack this turn" is an intervening-if clause, so the condition
    /// is checked when the ability would trigger and not only when it
    /// resolves. Without this the trigger went on the stack even when the
    /// Brute had attacked — a stack entry the rules say never exists, and a
    /// priority window with it.
    fn should_trigger(&self, state: &GameState, self_id: ObjectId, kind: &TriggerKind, _registry: &CardRegistry) -> bool {
        match kind {
            TriggerKind::EndStep => !state.attacked_this_turn(self_id),
            _ => true,
        }
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], _registry: &CardRegistry) {
        let is_transformed = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.is_transformed,
            _ => return,
        };
        // `step_trigger_scope` already scopes this to the controller's own end
        // step; re-deriving that here is duplication, not defence.
        if !is_transformed {
            return;
        }
        // The condition is checked a second time on resolution (CR 603.4).
        if !state.attacked_this_turn(self_id) {
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.tapped = true; // "tap Homicidal Brute, then transform it"
            }
            // Through the helper rather than flipping the flag by hand, so
            // this cannot drift from what transforming means.
            crate::cards::helpers::apply_transform(state, self_id, _registry);
            state.log(crate::state::LogLevel::Event,
                "Homicidal Brute transforms back into Civilized Scholar (didn't attack)".into());
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

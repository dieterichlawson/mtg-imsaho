use crate::actions::Target;
use crate::cards::{AttackInfo, ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
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
/// The draw-discard is implemented as an activated ability. After drawing, the player
/// chooses which card to discard. If the discarded card is a creature, Civilized Scholar
/// untaps and transforms into Homicidal Brute. The end-step transform-back checks
/// `card_state` for whether it attacked.
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

    /// Whether this permanent attacked during the current turn.
    ///
    /// The marker is stamped with the turn it happened on rather than being a
    /// bare flag: a bare marker set by a front-face attack in an earlier turn
    /// stayed set forever, and the next transform read it as "attacked" and
    /// refused to flip back. CR 711.5 — transforming does not make a new
    /// object, so an attack made as Civilized Scholar counts for Homicidal
    /// Brute in the same turn (Scryfall ruling, 2011-09-22).
    fn attacked_this_turn(state: &GameState, self_id: ObjectId) -> bool {
        let this_turn = crate::ids::ObjectId(u64::from(state.turn_number));
        state.get_object(self_id)
            .and_then(|o| o.card_state.get("attacked_on_turn").copied())
            == Some(this_turn)
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
            // Front face: Attacks trigger is only here for internal state tracking
            // (marking that the creature attacked this turn so Homicidal Brute's
            // end-step check can see it). Per Scryfall ruling [2011-09-22] attacks
            // count regardless of face, so we keep the Attacks trigger on the front
            // face too. The real oracle trigger (EndStep transform-back) lives on
            // the back face (Homicidal Brute) where it belongs.
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "mark as attacked this turn".into(),
                target_requirement: None,
                },
            ],
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
                // Also track attacks on the back face so Homicidal Brute's own
                // attacks count toward the "didn't attack" check.
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "mark as attacked this turn".into(),
                target_requirement: None,
                },
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

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Option<(i32, i32)> {
        if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
            Some((5, 1))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
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
            counter_cost: None,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
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
                crate::cards::helpers::apply_transform(state, object_id, registry);
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.tapped = false; // Scholar untaps on transform per oracle
                }
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
            crate::cards::helpers::apply_transform(state, self_id, registry);
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.tapped = false; // Scholar untaps on transform per oracle
            }
            state.log(crate::state::LogLevel::Event,
                "Civilized Scholar transforms into Homicidal Brute".into());
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _attack: AttackInfo, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // Mark that we attacked this turn (so end-step doesn't transform back).
        let turn = state.turn_number;
        if let Some(obj) = state.get_object_mut(self_id) {
            // Stamped with the turn number, not a bare marker. The clearing
            // path below only runs on the BACK face's end step, so a front-face
            // attack in an earlier turn left a bare marker set forever — and
            // the next time this transformed, its end-step trigger read that
            // stale marker and refused to transform back.
            obj.card_state.insert("attacked_on_turn".into(), crate::ids::ObjectId(u64::from(turn)));
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
            TriggerKind::EndStep => !Self::attacked_this_turn(state, self_id),
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
        if !Self::attacked_this_turn(state, self_id) {
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

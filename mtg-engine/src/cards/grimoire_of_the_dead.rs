use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::*;

/// Grimoire of the Dead {4} Legendary Artifact.
/// {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
/// {T}, Remove three study counters from Grimoire of the Dead and sacrifice it:
/// Put all creature cards from all graveyards onto the battlefield under your control.
/// They're black Zombies in addition to their other colors and types.
///
/// Simplified: Discard ability auto-picks a card from hand. Study counters tracked via
/// card_state. The sacrifice ability returns all graveyard creatures.
pub struct GrimoireOfTheDead;

impl CardBehavior for GrimoireOfTheDead {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grimoire of the Dead".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.\n{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_legendary = true;
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        let controller = obj.controller;
        let study_counters = obj.card_state.get("study_counters")
            .map(|id| id.0 as u32)
            .unwrap_or(0);
        let has_cards_in_hand = !state.objects_in_zone(Zone::Hand, controller).is_empty();

        let mut abilities = vec![];

        // Ability 0: {1}, {T}, Discard a card: Put a study counter.
        if has_cards_in_hand {
            abilities.push(ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}, {T}, Discard: Put a study counter on Grimoire".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            });
        }

        // Ability 1: {T}, Remove 3 study counters, sacrifice: Return all creatures from graveyards.
        if study_counters >= 3 {
            abilities.push(ActivatedAbilityDef {
                ability_index: 1,
                description: "{T}, Remove 3 study counters, sacrifice: Return all graveyard creatures".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::SacrificeThis,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            });
        }

        abilities
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        match ability_index {
            0 => {
                // Discard a card, add a study counter.
                let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, controller)
                    .iter().map(|o| o.id).collect();

                if hand.len() == 1 {
                    grimoire_discard_and_counter(state, object_id, hand[0], controller);
                } else if hand.len() > 1 {
                    // Present choice of which card to discard.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: controller,
                        source: object_id,
                        choice: ResolutionChoiceKind::ChooseCardFromHand {
                            player: controller,
                            description: "Grimoire of the Dead: choose a card to discard".into(),
                            cards: hand,
                            callback_source: Some(object_id),
                        },
                    });
                }
            }
            1 => {
                // Remove 3 study counters, sacrifice Grimoire, return all graveyard creatures.
                // Sacrifice Grimoire.
                crate::destruction::sacrifice(state, object_id, registry);

                // Collect all creature cards from all graveyards.
                let creatures: Vec<(ObjectId, crate::ids::PlayerId)> = state.objects.values()
                    .filter(|o| o.zone == Zone::Graveyard && o.power.is_some())
                    .map(|o| (o.id, o.owner))
                    .collect();

                let count = creatures.len();
                for (cid, _owner) in creatures {
                    let name = state.get_object(cid).map(|o| o.name.clone()).unwrap_or_default();
                    state.move_object(cid, Zone::Battlefield);
                    if let Some(obj) = state.get_object_mut(cid) {
                        obj.controller = controller;
                        // They're black Zombies in addition to their other types.
                        if !obj.subtypes.contains(&"Zombie".into()) {
                            obj.subtypes.push("Zombie".into());
                        }
                        if !obj.colors.contains(&Color::Black) {
                            obj.colors.push(Color::Black);
                        }
                    }
                    state.log(crate::state::LogLevel::Event,
                        format!("Grimoire of the Dead: {} returned as a Zombie", name));
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Grimoire of the Dead: {} creatures returned from all graveyards", count));
            }
            _ => {}
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, discarded_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(self_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        grimoire_discard_and_counter(state, self_id, discarded_id, controller);
    }
}

fn grimoire_discard_and_counter(state: &mut GameState, grimoire_id: ObjectId, card_id: ObjectId, controller: crate::ids::PlayerId) {
    let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
    state.move_object(card_id, Zone::Graveyard);
    state.events.push(crate::events::GameEvent::Discarded {
        player: controller,
        object: card_id,
    });
    state.log(crate::state::LogLevel::Event,
        format!("Grimoire of the Dead: p{} discarded {}", controller.0, name));

    let current = state.get_object(grimoire_id)
        .and_then(|o| o.card_state.get("study_counters"))
        .map(|id| id.0 as u32)
        .unwrap_or(0);
    if let Some(obj) = state.get_object_mut(grimoire_id) {
        obj.card_state.insert("study_counters".into(), ObjectId((current + 1) as u64));
    }
    state.log(crate::state::LogLevel::Event,
        format!("Grimoire of the Dead: study counter added ({}/3)", current + 1));
}

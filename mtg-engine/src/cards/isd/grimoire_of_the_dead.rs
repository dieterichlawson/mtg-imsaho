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

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_legendary = true;
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        let controller = obj.controller;
        let study_counters = state.get_counter_count(object_id, CounterType::Study);
        let has_cards_in_hand = !state.objects_in_zone(Zone::Hand, controller).is_empty();

        let mut abilities = vec![];

        // Ability 0: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
        if has_cards_in_hand {
            abilities.push(ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}, {T}, Discard a card: Put a study counter on Grimoire".into(),
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
                // {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
                // Present the discard choice to the player.
                let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, controller)
                    .iter().map(|o| o.id).collect();

                if hand.is_empty() {
                    return;
                }

                if hand.len() == 1 {
                    // Only one card in hand -- auto-discard it.
                    let card_id = hand[0];
                    let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
                    state.move_object(card_id, Zone::Graveyard, registry);
                    state.events.push(crate::events::GameEvent::Discarded {
                        player: controller,
                        object: card_id,
                    });
                    state.log(crate::state::LogLevel::Event,
                        format!("Grimoire of the Dead: p{} discarded {}", controller.0, name));

                    // Add the study counter.
                    state.add_counters(object_id, CounterType::Study, 1);
                    let count = state.get_counter_count(object_id, CounterType::Study);
                    state.log(crate::state::LogLevel::Event,
                        format!("Grimoire of the Dead: study counter added ({}/3)", count));
                } else {
                    // Multiple cards -- present choice to player.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: controller,
                        source: object_id,
                        choice: ResolutionChoiceKind::ChooseCardFromHand {
                            description: "Grimoire of the Dead: choose a card to discard".into(),
                            player: controller,
                            cards: hand,
                        },
                    });
                }
            }
            1 => {
                // {T}, Remove 3 study counters, sacrifice: Return all graveyard creatures.
                // Note: The engine already handled tapping and sacrificing as part of the cost.
                // The study counters were already checked in activated_abilities().
                // (Since the Grimoire is now in the graveyard, counter removal is moot.)

                // Collect all creature cards from all graveyards.
                // "Creature cards" includes each card with the type creature, even if
                // it has additional types (ruling 2011-09-22). We check card_types
                // for Creature in addition to checking power (which is the heuristic
                // for creatures created by the engine).
                let creatures: Vec<ObjectId> = state.objects.values()
                    .filter(|o| o.zone == Zone::Graveyard && o.id != object_id)
                    .filter(|o| {
                        o.power.is_some() || o.card_types.contains(&CardType::Creature)
                    })
                    .map(|o| o.id)
                    .collect();

                let count = creatures.len();
                for cid in creatures {
                    let (name, is_legendary) = state.get_object(cid)
                        .map(|o| {
                            let legendary = registry.card_data(o.card_id)
                                .map(|d| d.supertypes.contains(&Supertype::Legendary))
                                .unwrap_or(false);
                            (o.name.clone(), legendary)
                        })
                        .unwrap_or_else(|| (String::new(), false));
                    state.move_object(cid, Zone::Battlefield, registry);
                    if let Some(obj) = state.get_object_mut(cid) {
                        obj.controller = controller;
                        obj.is_legendary = is_legendary;
                        // They're black Zombies in addition to their other colors and types.
                        if !obj.subtypes.contains(&"Zombie".into()) {
                            obj.subtypes.push("Zombie".into());
                        }
                        if !obj.colors.contains(&Color::Black) {
                            obj.colors.push(Color::Black);
                        }
                    }
                    state.log(crate::state::LogLevel::Event,
                        format!("Grimoire of the Dead: {} returned as a black Zombie", name));
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Grimoire of the Dead: {} creatures returned from all graveyards", count));
            }
            _ => {}
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, _discarded_id: ObjectId, registry: &CardRegistry) {
        // After the player chooses a card to discard, add a study counter to the Grimoire.
        state.add_counters(self_id, CounterType::Study, 1);
        let count = state.get_counter_count(self_id, CounterType::Study);
        state.log(crate::state::LogLevel::Event,
            format!("Grimoire of the Dead: study counter added ({}/3)", count));
    }
}

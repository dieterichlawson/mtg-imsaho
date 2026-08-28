use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, CardType, Supertype, Zone, CounterType, Color};

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
            oracle_text: "{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.\n{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        let controller = obj.controller;
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
                counter_cost: None,
            });
        }

        // Ability 1: {T}, Remove 3 study counters, sacrifice: Return all creatures from graveyards.
        // The engine checks the counters are there and removes exactly three
        // before the sacrifice, which would otherwise clear all of them at once.
        abilities.push(ActivatedAbilityDef {
            ability_index: 1,
            description: "{T}, Remove 3 study counters, sacrifice: Return all graveyard creatures".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: Some((CounterType::Study, 3)),
        });

        abilities
    }

    /// "{1}, {T}, Discard a card:" — everything before the colon is cost, so
    /// the discard is paid on activation (CR 601.2h via 602.2b), not when the
    /// ability resolves. It used to happen in `resolve_activated_ability`,
    /// which put it on the wrong side of the priority window: an opponent
    /// responding to the ability still saw the card in hand, and countering
    /// the ability took the discard back with it.
    fn pay_activation_cost(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        if ability_index != 0 {
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, object_id);
        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, controller)
            .iter().map(|o| o.id).collect();

        match hand.len() {
            // `activated_abilities` does not offer the ability with an empty
            // hand, because a cost that cannot be paid cannot be chosen
            // (CR 601.2h).
            0 => {}
            1 => {
                let name = state.obj_name(hand[0]);
                state.discard_card(hand[0], registry);
                state.log(crate::state::LogLevel::Event,
                    format!("Grimoire of the Dead: p{} discarded {name} to pay the cost", controller.0));
            }
            _ => {
                // Which card to discard is the player's choice, made while
                // paying (CR 601.2b).
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: object_id,
                    choice: ResolutionChoiceKind::ChooseCardFromHand {
                        description: "Grimoire of the Dead: choose a card to discard".into(),
                        player: controller,
                        cards: hand,
                        discard_immediately: true,
                    },
                });
            }
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        match ability_index {
            0 => {
                // The discard was the cost; this is the effect.
                state.add_counters(object_id, CounterType::Study, 1);
                let count = state.get_counter_count(object_id, CounterType::Study);
                state.log(crate::state::LogLevel::Event,
                    format!("Grimoire of the Dead: study counter added ({count}/3)"));
            }
            1 => {
                // {T}, Remove 3 study counters, sacrifice: Return all graveyard creatures.
                // Tap, counter removal and sacrifice are all cost, all paid by
                // the engine before this runs.

                // Collect all creature cards from all graveyards.
                // "Creature cards" includes each card with the type creature, even if
                // it has additional types (ruling 2011-09-22). We check card_types
                // for Creature in addition to checking power (which is the heuristic
                // for creatures created by the engine).
                let creatures: Vec<ObjectId> = state.all_objects_in_zone(Zone::Graveyard).into_iter()
                    .filter(|o| o.id != object_id && state.is_card(o.id))
                    .filter(|o| {
                        state.is_creature(o.id, registry)
                    })
                    .map(|o| o.id)
                    .collect();

                let count = creatures.len();
                for cid in creatures {
                    let name = state.obj_name(cid);
                    state.move_object_under_control(cid, Zone::Battlefield, controller, registry);
                    // No `is_legendary` stamping here any more: the legend rule
                    // reads the active face (`state.is_legendary`), so a
                    // reanimated legend is caught by CR 704.5j without every
                    // card that puts one onto the battlefield remembering to
                    // say so.
                    if let Some(obj) = state.get_object_mut(cid) {
                        // They're black Zombies in addition to their other colors and types.
                        if !obj.subtypes.contains(&"Zombie".into()) {
                            obj.subtypes.push("Zombie".into());
                        }
                        if !obj.colors.contains(&Color::Black) {
                            obj.colors.push(Color::Black);
                        }
                    }
                    state.log(crate::state::LogLevel::Event,
                        format!("Grimoire of the Dead: {name} returned as a black Zombie"));
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Grimoire of the Dead: {count} creatures returned from all graveyards"));
            }
            _ => {}
        }
    }

}

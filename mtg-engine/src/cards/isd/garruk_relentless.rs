use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef, TargetRequirement};
use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaCost, ManaSymbol, Color, Supertype, CounterType, Keyword};

/// Garruk Relentless {3}{G} Legendary Planeswalker — Garruk (3 loyalty).
///
/// Front face (Garruk Relentless):
///   When Garruk Relentless has two or fewer loyalty counters on him, transform him.
///   0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power
///      to Garruk.
///   0: Create a 2/2 green Wolf creature token.
///
/// Back face (Garruk, the Veil-Cursed):
///   +1: Create a 1/1 black Wolf creature token with deathtouch.
///   -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it,
///       put it into your hand, then shuffle.
///   -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the
///       number of creature cards in your graveyard.
pub struct GarrukRelentless;

impl GarrukRelentless {
    /// Sacrifice a creature and then search library for a creature card.
    /// Used when there's only one creature to sacrifice (no choice needed).
    fn sacrifice_and_tutor(state: &mut GameState, garruk_id: ObjectId, sac_id: ObjectId, controller: crate::ids::PlayerId, registry: &CardRegistry) {
        use rand::seq::SliceRandom;

        let sac_name = state.get_object(sac_id).map(|o| o.name.clone()).unwrap_or_default();
        crate::destruction::sacrifice(state, sac_id, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Garruk, the Veil-Cursed: sacrificed {sac_name}"));

        // Find all creature cards in library for the player to choose from.
        let creature_options: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&lib_id| {
                if let Some(obj) = state.get_object(lib_id) {
                    if obj.card_types.is_empty() {
                        registry.card_data(obj.card_id)
                            .is_some_and(|d| d.card_types.contains(&CardType::Creature))
                    } else {
                        obj.card_types.contains(&CardType::Creature)
                    }
                } else {
                    false
                }
            })
            .copied()
            .collect();

        if creature_options.is_empty() {
            state.log(crate::state::LogLevel::Event,
                "Garruk, the Veil-Cursed: no creature card found in library".into());
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
        } else if creature_options.len() == 1 {
            let found_id = creature_options[0];
            let found_name = state.get_object(found_id).map(|o| o.name.clone()).unwrap_or_default();
            let player = state.get_player_mut(controller);
            player.library_order.retain(|&lid| lid != found_id);
            state.move_object(found_id, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Garruk, the Veil-Cursed: searched and found {found_name}"));
            let mut rng = rand::thread_rng();
            state.get_player_mut(controller).library_order.shuffle(&mut rng);
        } else {
            // Multiple creatures in library — present choice.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: garruk_id,
                choice: ResolutionChoiceKind::ChooseFromLibrary {
                    description: "Garruk, the Veil-Cursed: choose a creature card from your library".into(),
                    options: creature_options,
                    searcher: controller,
                    source_id: garruk_id,
                },
            });
        }
    }
}

impl CardBehavior for GarrukRelentless {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Garruk Relentless".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Planeswalker],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Garruk".into()],
            power: None,
            toughness: None,
            oracle_text: "When Garruk has two or fewer loyalty counters on him, transform him.\n0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.\n0: Create a 2/2 green Wolf creature token.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn starting_loyalty(&self) -> Option<u32> {
        Some(3)
    }

    fn loyalty_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<LoyaltyAbilityDef> {
        let is_transformed = state.get_object(object_id).is_some_and(|o| o.is_transformed);

        if is_transformed {
            // Back face: Garruk, the Veil-Cursed
            vec![
                LoyaltyAbilityDef {
                    ability_index: 10,
                    loyalty_change: 1,
                    description: "+1: Create a 1/1 black Wolf with deathtouch".into(),
                    target_requirement: None,
                },
                LoyaltyAbilityDef {
                    ability_index: 11,
                    loyalty_change: -1,
                    description: "-1: Sacrifice a creature, search library for a creature card".into(),
                    target_requirement: None,
                },
                LoyaltyAbilityDef {
                    ability_index: 12,
                    loyalty_change: -3,
                    description: "-3: Creatures you control get +X/+X and trample (X = creature cards in graveyard)".into(),
                    target_requirement: None,
                },
            ]
        } else {
            // Front face: Garruk Relentless
            vec![
                LoyaltyAbilityDef {
                    ability_index: 0,
                    loyalty_change: 0,
                    description: "0: Deal 3 damage to target creature, it fights back".into(),
                    target_requirement: Some(TargetRequirement::Creature),
                },
                LoyaltyAbilityDef {
                    ability_index: 1,
                    loyalty_change: 0,
                    description: "0: Create a 2/2 Wolf token".into(),
                    target_requirement: None,
                },
            ]
        }
    }

    fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };

        match ability_index {
            // ── Front face abilities ─────────────────────────────────────
            0 => {
                // 0: Garruk deals 3 damage to target creature. That creature deals damage
                // equal to its power to him.
                if let Some(Target::Object(target_id)) = targets.first() {
                    let target_power = state.effective_power(*target_id, registry).unwrap_or(0);
                    let target_name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
                    // Deal 3 to the creature.
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        obj.damage_marked += 3;
                        obj.damaged_by.push(self_id);
                    }
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: self_id,
                        target: crate::events::DamageTarget::Object(*target_id),
                        amount: 3,
                    });
                    // The creature deals its power as damage to Garruk (remove loyalty counters).
                    if target_power > 0 {
                        let remove = u32::try_from(target_power).unwrap_or(0);
                        if let Some(obj) = state.get_object_mut(self_id) {
                            let loyalty = obj.counters.entry(CounterType::Loyalty).or_insert(0);
                            *loyalty = loyalty.saturating_sub(remove);
                        }
                        state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                            source: *target_id,
                            target: crate::events::DamageTarget::Object(self_id),
                            amount: remove,
                        });
                    }
                    state.log(crate::state::LogLevel::Event,
                        format!("Garruk: deals 3 to {target_name}, takes {target_power} damage back"));
                }
            }
            1 => {
                // 0: Create a 2/2 green Wolf token.
                state.create_token_with_subtypes(
                    "Wolf",
                    controller,
                    2, 2,
                    vec![Color::Green],
                    vec![CardType::Creature],
                    vec![],
                    vec!["Wolf".into()],
                    registry,
                );
                state.log(crate::state::LogLevel::Event,
                    "Garruk: created a 2/2 Wolf token".into());
            }

            // ── Back face abilities (Garruk, the Veil-Cursed) ────────────
            10 => {
                // +1: Create a 1/1 black Wolf creature token with deathtouch.
                state.create_token_with_subtypes(
                    "Wolf",
                    controller,
                    1, 1,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![Keyword::Deathtouch],
                    vec!["Wolf".into()],
                    registry,
                );
                state.log(crate::state::LogLevel::Event,
                    "Garruk, the Veil-Cursed: created a 1/1 black Wolf token with deathtouch".into());
            }
            11 => {
                // -1: Sacrifice a creature. If you do, search your library for a creature card,
                // reveal it, put it into your hand, then shuffle.
                // Per ruling: "doesn't target a creature. However, when that ability resolves,
                // you must sacrifice a creature if you control one."
                let creatures: Vec<Target> = state.objects_in_zone(Zone::Battlefield, controller)
                    .iter()
                    .filter(|o| o.card_types.contains(&CardType::Creature)
                        || o.power.is_some()) // creatures include tokens
                    .map(|o| Target::Object(o.id))
                    .collect();

                if creatures.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        "Garruk, the Veil-Cursed: no creature to sacrifice".into());
                } else if creatures.len() == 1 {
                    // Only one creature — auto-sacrifice and tutor.
                    if let Target::Object(sac_id) = creatures[0] {
                        Self::sacrifice_and_tutor(state, self_id, sac_id, controller, registry);
                    }
                } else {
                    // Multiple creatures — present choice to player.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: controller,
                        source: self_id,
                        choice: ResolutionChoiceKind::ChooseTarget {
                            description: "Garruk, the Veil-Cursed: choose a creature to sacrifice".into(),
                            options: creatures,
                            optional: false,
                            effect: PendingEffect::SacrificeAndTutor { garruk_id: self_id },
                        },
                    });
                }
            }
            12 => {
                // -3: Creatures you control gain trample and get +X/+X until end of turn,
                // where X is the number of creature cards in your graveyard.
                let x = i32::try_from(state.objects_in_zone(Zone::Graveyard, controller)
                    .iter()
                    .filter(|o| {
                        if o.card_types.is_empty() {
                            registry.card_data(o.card_id)
                                .is_some_and(|d| d.card_types.contains(&CardType::Creature))
                        } else {
                            o.card_types.contains(&CardType::Creature)
                        }
                    })
                    .count()).unwrap_or(i32::MAX);

                let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
                    .iter()
                    .filter(|o| o.card_types.contains(&CardType::Creature))
                    .map(|o| o.id)
                    .collect();

                for cid in &creatures {
                    state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                        target: *cid,
                        power_mod: x,
                        toughness_mod: x,
                    });
                    state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantKeyword {
                        target: *cid,
                        keyword: Keyword::Trample,
                    });
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Garruk, the Veil-Cursed: creatures get +{x}/+{x} and trample until end of turn"));
            }
            _ => {}
        }

    }

    fn on_state_trigger(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        // State-triggered ability (CR 603.8): transform Garruk Relentless into
        // Garruk, the Veil-Cursed when he has 2 or fewer loyalty counters.
        if let Some(obj) = state.get_object_mut(self_id) {
            if !obj.is_transformed {
                obj.is_transformed = true;
                obj.name = "Garruk, the Veil-Cursed".into();
                state.log(crate::state::LogLevel::Event,
                    "Garruk Relentless transforms into Garruk, the Veil-Cursed".into());
            }
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[crate::actions::Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        state.add_counters(object_id, CounterType::Loyalty, 3);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.card_types = vec![CardType::Planeswalker];
            obj.is_legendary = true;
        }
        state.log(crate::state::LogLevel::Event,
            "Garruk Relentless enters with 3 loyalty".into());
    }
}

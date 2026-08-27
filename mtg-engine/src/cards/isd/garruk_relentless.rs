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
            oracle_text: "When Garruk has two or fewer loyalty counters on him, transform him.\n0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.\n0: Create a 2/2 green Wolf creature token.".into(),
            ..Default::default()
        }
    }

    /// Garruk, the Veil-Cursed. He has no triggered abilities and no P/T; what
    /// the back face carries that matters is his *name* and his rules text, and
    /// those are what `face_data` hands to `name_of`, the log, and the legend
    /// rule. Declaring the face is the only way to say so — this card used to
    /// model its back face purely by branching on `is_transformed` in
    /// `loyalty_abilities`, and wrote the name into `obj.name` by hand on
    /// transform, so every authoritative read still answered "Garruk
    /// Relentless" with the front face's oracle text.
    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Garruk, the Veil-Cursed".into(),
            card_types: vec![CardType::Planeswalker],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Garruk".into()],
            oracle_text: "+1: Create a 1/1 black Wolf creature token with deathtouch.\n−1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.\n−3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.".into(),
            ..Default::default()
        })
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
                    let self_name = state.get_object(self_id).map(|o| o.name.clone()).unwrap_or_default();

                    let garruk_effect = PendingEffect::DealDamage {
                        amount: 3,
                        source_id: self_id,
                        source_name: self_name,
                    };
                    crate::engine::apply_pending_effect(state, &Target::Object(*target_id), &garruk_effect, registry);

                    if target_power > 0 {
                        let remove = u32::try_from(target_power).unwrap_or(0);
                        let creature_effect = PendingEffect::DealDamage {
                            amount: remove,
                            source_id: *target_id,
                            source_name: target_name.clone(),
                        };
                        crate::engine::apply_pending_effect(state, &Target::Object(self_id), &creature_effect, registry);
                    }
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
                    .filter(|o| state.is_creature(o.id, registry))
                    .map(|o| Target::Object(o.id))
                    .collect();

                if creatures.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        "Garruk, the Veil-Cursed: no creature to sacrifice".into());
                } else if creatures.len() == 1 {
                    // One creature and the sacrifice is mandatory, so there is
                    // nothing to choose — but it is the same effect, through
                    // the same code. This branch used to carry its own copy,
                    // and the two had already drifted apart on how they read
                    // the sacrificed creature's name.
                    self.resolve_card_effect(state, self_id, "", &creatures[0], registry);
                } else {
                    // Multiple creatures — present choice to player.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: controller,
                        source: self_id,
                        choice: ResolutionChoiceKind::ChooseTarget {
                            description: "Garruk, the Veil-Cursed: choose a creature to sacrifice".into(),
                            options: creatures,
                            optional: false,
                            effect: PendingEffect::CardEffect { source_id: self_id, key: String::new() },
                        },
                    });
                }
            }
            12 => {
                // -3: Creatures you control gain trample and get +X/+X until end of turn,
                // where X is the number of creature cards in your graveyard.
                let x = i32::try_from(state.objects_in_zone(Zone::Graveyard, controller)
                    .iter()
                    // "the number of creature **cards** in your graveyard" —
                    // CR 109.1, so a token that has just died does not count.
                    .filter(|o| state.is_card(o.id) && state.is_creature(o.id, registry))
                    .count()).unwrap_or(i32::MAX);

                // `obj.card_types` is empty for every non-token permanent, so
                // reading it directly silently excluded ordinary creatures from
                // the buff. `is_creature` resolves through the active face.
                let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
                    .iter()
                    .filter(|o| state.is_creature(o.id, registry))
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

    /// CR 603.8: "When Garruk Relentless has two or fewer loyalty counters on
    /// him, transform him." The threshold is this card's text.
    fn state_trigger_condition(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        state.get_object(object_id).is_some_and(|o| {
            !o.is_transformed
                && *o.counters.get(&CounterType::Loyalty).unwrap_or(&0) <= 2
        })
    }

    fn state_trigger_description(&self) -> String {
        "When Garruk Relentless has two or fewer loyalty counters on him, transform him".into()
    }

    fn on_state_trigger(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        // State-triggered ability (CR 603.8): transform Garruk Relentless into
        // Garruk, the Veil-Cursed when he has 2 or fewer loyalty counters.
        //
        // Through the shared helper, not by flipping the flag and writing the
        // name by hand — the name comes from the face now, so there is one
        // definition of what transforming does. Ruling (2011-09-22): no loyalty
        // counters are added or removed by the transform, and the helper
        // touches none.
        if state.get_object(self_id).is_some_and(|o| o.is_transformed) {
            return;
        }
        crate::cards::helpers::apply_transform(state, self_id, registry);
        state.log(crate::state::LogLevel::Event,
            "Garruk Relentless transforms into Garruk, the Veil-Cursed".into());
    }

    /// Garruk, the Veil-Cursed -1: "Sacrifice a creature. Search your library
    /// for a creature card, reveal it, put it into your hand, then shuffle."
    /// The sacrifice, the search and the shuffle are all this card's text.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let controller = crate::cards::helpers::controller_of(state, source_id);

        let sac_name = state.obj_name(*id);
        crate::destruction::sacrifice(state, *id, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Garruk, the Veil-Cursed: sacrificed {sac_name}"));

        let creature_options: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .copied()
            .filter(|&lib_id| state.has_card_type(lib_id, CardType::Creature, registry))
            .collect();

        // "Search your library for a creature card, reveal it, put it into
        // your hand, then shuffle."
        crate::cards::helpers::search_library(
            state, source_id, controller, creature_options,
            Zone::Hand, false, false,
            "Garruk, the Veil-Cursed: choose a creature card from your library",
            registry,
        );
    }
}

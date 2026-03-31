use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Garruk Relentless {3}{G} Planeswalker — Garruk (3 loyalty).
/// When Garruk Relentless has two or fewer loyalty counters on him, transform him.
/// 0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power
///    to Garruk.
/// 0: Create a 2/2 green Wolf creature token.
///
/// Simplified: Front face only. Back face (Garruk, the Veil-Cursed) is not implemented.
/// The transform condition is noted but the back face abilities are omitted.
/// When Garruk reaches 2 or fewer loyalty, he transforms (but we just log it; back face
/// abilities aren't available).
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
            supertypes: vec![],
            subtypes: vec!["Garruk".into()],
            power: None,
            toughness: None,
            oracle_text: "When Garruk Relentless has two or fewer loyalty counters on him, transform Garruk Relentless.\n0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him.\n0: Create a 2/2 green Wolf creature token.".into(),
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

    fn loyalty_abilities(&self) -> Vec<LoyaltyAbilityDef> {
        vec![
            LoyaltyAbilityDef {
                ability_index: 0,
                loyalty_change: 0,
                description: "0: Deal 3 damage to target creature, it fights back".into(),
            },
            LoyaltyAbilityDef {
                ability_index: 1,
                loyalty_change: 0,
                description: "0: Create a 2/2 Wolf token".into(),
            },
        ]
    }

    fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        let opponent = state.opponent(controller);

        match ability_index {
            0 => {
                // 0: Deal 3 damage to target creature. That creature deals damage equal to its
                // power to Garruk. Simplified: pick the strongest opponent creature.
                let target: Option<(ObjectId, i32)> = state.objects_in_zone(Zone::Battlefield, opponent)
                    .iter()
                    .filter(|o| o.power.is_some())
                    .map(|o| (o.id, state.effective_power(o.id, registry).unwrap_or(0)))
                    .max_by_key(|(_, p)| *p);

                if let Some((target_id, target_power)) = target {
                    let target_name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                    // Deal 3 to the creature.
                    if let Some(obj) = state.get_object_mut(target_id) {
                        obj.damage_marked += 3;
                        obj.damaged_by.push(self_id);
                    }
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: self_id,
                        target: crate::events::DamageTarget::Object(target_id),
                        amount: 3,
                    });
                    // The creature deals its power as damage to Garruk (remove loyalty counters).
                    if target_power > 0 {
                        let remove = target_power as u32;
                        if let Some(obj) = state.get_object_mut(self_id) {
                            let loyalty = obj.counters.entry(CounterType::Loyalty).or_insert(0);
                            *loyalty = loyalty.saturating_sub(remove);
                        }
                    }
                    state.log(crate::state::LogLevel::Event,
                        format!("Garruk: deals 3 to {}, takes {} damage back", target_name, target_power));
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
                );
                state.log(crate::state::LogLevel::Event,
                    "Garruk: created a 2/2 Wolf token".into());
            }
            _ => {}
        }

        // Check transform condition: 2 or fewer loyalty → transform.
        let loyalty = state.get_counter_count(self_id, CounterType::Loyalty);
        if loyalty <= 2 {
            if let Some(obj) = state.get_object_mut(self_id) {
                if !obj.is_transformed {
                    obj.is_transformed = true;
                    obj.name = "Garruk, the Veil-Cursed".into();
                    state.log(crate::state::LogLevel::Event,
                        "Garruk Relentless transforms into Garruk, the Veil-Cursed (back face abilities not implemented)".into());
                }
            }
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[crate::actions::Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        state.add_counters(object_id, CounterType::Loyalty, 3);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.card_types = vec![CardType::Planeswalker];
        }
        state.log(crate::state::LogLevel::Event,
            "Garruk Relentless enters with 3 loyalty".into());
    }
}

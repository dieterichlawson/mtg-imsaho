use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Paraselene — {2}{W} Sorcery.
/// Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
pub struct Paraselene;

impl CardBehavior for Paraselene {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Paraselene".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Find all enchantments on the battlefield.
        let enchantments: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield)
            .filter(|o| {
                registry.card_data(o.card_id)
                    .is_some_and(|d| d.card_types.contains(&CardType::Enchantment))
            })
            .map(|o| o.id)
            .collect();

        let mut destroyed_count = 0u32;
        for eid in enchantments {
            let result = crate::destruction::try_destroy(state, eid, registry);
            if result == crate::destruction::DestroyResult::Died {
                destroyed_count += 1;
            }
        }

        if destroyed_count > 0 {
            let old_life = state.get_player(controller).life;
            let new_life = old_life + i32::try_from(destroyed_count).unwrap_or(i32::MAX);
            state.get_player_mut(controller).life = new_life;
            state.events.push(crate::events::GameEvent::LifeChanged {
                player: controller,
                old: old_life,
                new_life,
            });
            state.log(crate::state::LogLevel::Event,
                format!("Paraselene destroyed {} enchantments, p{} gained {} life",
                    destroyed_count, controller.0, destroyed_count));
        } else {
            state.log(crate::state::LogLevel::Event,
                "Paraselene: no enchantments to destroy".into());
        }

    }
}

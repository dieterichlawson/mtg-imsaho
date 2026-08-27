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
            oracle_text: "Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Find all enchantments on the battlefield.
        let enchantments: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield)
            .filter(|o| {
                state.face_data(o.id, registry)
                    .is_some_and(|d| d.card_types.contains(&CardType::Enchantment))
            })
            .map(|o| o.id)
            .collect();

        // "Destroy all enchantments" — one event (CR 700.2c), so the
        // indestructible check for each is made against the battlefield as it
        // was before any of them died.
        let destroyed_count = u32::try_from(
            crate::destruction::try_destroy_all(state, &enchantments, registry)
                .iter()
                .filter(|(_, r)| *r == crate::destruction::DestroyResult::Died)
                .count()).unwrap_or(u32::MAX);

        if destroyed_count > 0 {
            state.change_life(controller, i32::try_from(destroyed_count).unwrap_or(i32::MAX));
            state.log(crate::state::LogLevel::Event,
                format!("Paraselene destroyed {} enchantments, p{} gained {} life",
                    destroyed_count, controller.0, destroyed_count));
        } else {
            state.log(crate::state::LogLevel::Event,
                "Paraselene: no enchantments to destroy".into());
        }

    }
}

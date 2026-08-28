use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Blasphemous Act — {8}{R} Sorcery.
/// This spell costs {1} less to cast for each creature on the battlefield.
/// Blasphemous Act deals 13 damage to each creature.
pub struct BlasphemousAct;

impl CardBehavior for BlasphemousAct {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Blasphemous Act".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(8),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "This spell costs {1} less to cast for each creature on the battlefield.\nBlasphemous Act deals 13 damage to each creature.".into(),
            ..Default::default()
        }
    }

    fn modified_cost(&self, state: &GameState, registry: &CardRegistry) -> Option<ManaCost> {
        let creature_count = state.all_objects_in_zone(Zone::Battlefield).into_iter()
            .filter(|o| state.is_creature(o.id, registry))
            .count();
        let reduction = creature_count.min(8); // can't reduce below {R}
        let generic = 8u32.saturating_sub(u32::try_from(reduction).unwrap_or(u32::MAX));
        if generic == 8 {
            return None; // no reduction, use normal cost
        }
        Some(ManaCost::new(vec![ManaSymbol::Generic(generic), ManaSymbol::Colored(Color::Red)]))
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let creatures: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield).into_iter()
            .filter(|o| state.is_creature(o.id, registry))
            .map(|o| o.id)
            .collect();
        for id in creatures {
            let effect = PendingEffect::DealDamage {
                amount: 13,
                source_id: object_id,
            };
            crate::engine::apply_pending_effect(
                state,
                &Target::Object(id),
                &effect,
                registry,
            );
        }
        state.log(crate::state::LogLevel::Event,
            "Blasphemous Act deals 13 damage to each creature".into());
    }
}

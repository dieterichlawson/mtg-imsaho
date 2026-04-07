use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::events::{DamageTarget, GameEvent};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "This spell costs {1} less to cast for each creature on the battlefield.\nBlasphemous Act deals 13 damage to each creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn modified_cost(&self, state: &GameState, registry: &CardRegistry) -> Option<ManaCost> {
        let creature_count = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
            .count();
        let reduction = creature_count.min(8); // can't reduce below {R}
        let generic = 8u32.saturating_sub(reduction as u32);
        if generic == 8 {
            return None; // no reduction, use normal cost
        }
        Some(ManaCost::new(vec![ManaSymbol::Generic(generic), ManaSymbol::Colored(Color::Red)]))
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let creatures: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
            .map(|o| o.id)
            .collect();
        for id in creatures {
            if let Some(obj) = state.get_object_mut(id) {
                obj.damage_marked += 13;
                obj.damaged_by.push(object_id);
            }
            state.events.push(GameEvent::NonCombatDamageDealt {
                source: object_id,
                target: DamageTarget::Object(id),
                amount: 13,
            });
        }
        state.log(crate::state::LogLevel::Event,
            "Blasphemous Act deals 13 damage to each creature".into());
        state.move_spell_after_resolve(object_id, registry);
    }
}

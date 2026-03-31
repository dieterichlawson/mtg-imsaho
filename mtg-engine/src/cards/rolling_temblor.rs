use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Rolling Temblor — {2}{R} sorcery. Deals 2 damage to each creature without flying.
pub struct RollingTemblor;

impl CardBehavior for RollingTemblor {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rolling Temblor".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Rolling Temblor deals 2 damage to each creature without flying.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let creatures: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
            .map(|o| o.id)
            .collect();
        for id in creatures {
            if !state.has_keyword(id, Keyword::Flying, registry) {
                if let Some(obj) = state.get_object_mut(id) {
                    obj.damage_marked += 2;
                }
                state.events.push(crate::events::GameEvent::CombatDamageDealt {
                    source: object_id,
                    target: crate::events::DamageTarget::Object(id),
                    amount: 2,
                });
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

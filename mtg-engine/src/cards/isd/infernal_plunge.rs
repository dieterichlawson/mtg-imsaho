use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Infernal Plunge — {R} Sorcery.
/// As an additional cost to cast Infernal Plunge, sacrifice a creature.
/// Add {R}{R}{R}.
pub struct InfernalPlunge;

impl CardBehavior for InfernalPlunge {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Infernal Plunge".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::SacrificeCreature),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id)
            .map(|o| o.controller)
            .unwrap_or(crate::ids::PlayerId(0));

        // The creature sacrifice happens at cast time (as an additional cost).
        // On resolution, just add {R}{R}{R}.
        state.get_player_mut(controller).mana_pool.add(ManaType::Red, 3);
        state.move_spell_after_resolve(object_id);
    }
}

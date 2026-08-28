use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ManaType};

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
            oracle_text: "As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}.".into(),
            additional_cost: Some(AdditionalCost::SacrificeCreature),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // The creature sacrifice happens at cast time (as an additional cost).
        // On resolution, just add {R}{R}{R}.
        state.add_mana(controller, ManaType::Red, 3);
    }
}

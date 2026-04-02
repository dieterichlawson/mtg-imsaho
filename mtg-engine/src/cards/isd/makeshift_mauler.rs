use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Makeshift Mauler — {3}{U} 4/5 Zombie.
/// As an additional cost to cast Makeshift Mauler, exile a creature card from your graveyard.
pub struct MakeshiftMauler;

impl CardBehavior for MakeshiftMauler {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Makeshift Mauler".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Horror".into()],
            power: Some(4),
            toughness: Some(5),
            oracle_text: "As an additional cost to cast this spell, exile a creature card from your graveyard.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        // Additional cost (exile creature from graveyard) is handled at cast time by the engine.
        state.move_object(object_id, Zone::Battlefield);
    }
}

use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skaab Goliath — {5}{U} 6/9 Zombie Giant with Trample.
/// As an additional cost to cast Skaab Goliath, exile two creature cards from your graveyard.
pub struct SkaabGoliath;

impl CardBehavior for SkaabGoliath {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skaab Goliath".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Giant".into()],
            power: Some(6),
            toughness: Some(9),
            oracle_text: "Trample\nAs an additional cost to cast Skaab Goliath, exile two creature cards from your graveyard.".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(2)),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        // Additional cost (exile 2 creatures from graveyard) is handled at cast time by the engine.
        state.move_object(object_id, Zone::Battlefield);
    }
}

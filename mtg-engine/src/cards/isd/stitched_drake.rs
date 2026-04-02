use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Stitched Drake — {1}{U}{U} 3/4 Zombie Drake with Flying.
/// As an additional cost to cast Stitched Drake, exile a creature card from your graveyard.
pub struct StitchedDrake;

impl CardBehavior for StitchedDrake {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stitched Drake".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Drake".into()],
            power: Some(3),
            toughness: Some(4),
            oracle_text: "Flying\nAs an additional cost to cast Stitched Drake, exile a creature card from your graveyard.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        // The additional cost (exile a creature from graveyard) is handled by the
        // engine at cast time via AdditionalCost::ExileCreaturesFromGraveyard(1).
        state.move_object(object_id, Zone::Battlefield);
    }
}

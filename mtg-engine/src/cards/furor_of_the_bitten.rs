use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Furor of the Bitten — {R} aura enchantment. Enchanted creature gets +2/+2 and attacks each combat if able.
pub struct FurorOfTheBitten;

impl CardBehavior for FurorOfTheBitten {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Furor of the Bitten".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature gets +2/+2 and attacks each combat if able.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.move_object(object_id, Zone::Battlefield);
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.attached_to = Some(*target_id);
                    obj.summoning_sick = false;
                }
                return;
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

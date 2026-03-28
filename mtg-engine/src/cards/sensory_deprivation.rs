use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Sensory Deprivation — {U} aura enchantment. Enchanted creature gets -3/-0.
pub struct SensoryDeprivation;

impl CardBehavior for SensoryDeprivation {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sensory Deprivation".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature gets -3/-0.".into(),
            keywords: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
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

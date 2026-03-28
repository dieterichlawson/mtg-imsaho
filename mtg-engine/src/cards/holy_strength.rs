use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Holy Strength — {W} aura enchantment. Enchanted creature gets +1/+2.
pub struct HolyStrength;

impl CardBehavior for HolyStrength {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Holy Strength".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature gets +1/+2.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                // Move aura to battlefield and attach to target.
                state.move_object(object_id, Zone::Battlefield);
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.attached_to = Some(*target_id);
                    obj.summoning_sick = false; // enchantments don't have summoning sickness
                }
                return;
            }
        }
        // If target is invalid, aura goes to graveyard.
        state.move_object(object_id, Zone::Graveyard);
    }
}

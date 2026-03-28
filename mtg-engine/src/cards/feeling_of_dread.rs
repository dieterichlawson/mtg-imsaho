use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Feeling of Dread — {1}{W} instant. Tap up to two target creatures.
/// Simplified to tap one target creature.
pub struct FeelingOfDread;

impl CardBehavior for FeelingOfDread {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Feeling of Dread".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Tap up to two target creatures.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)])),
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(*target_id) {
                if obj.zone == Zone::Battlefield {
                    obj.tapped = true;
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

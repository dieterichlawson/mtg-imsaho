use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Nightbird's Clutches — {1}{R} sorcery. Up to two target creatures can't block this turn.
/// Simplified to one target; taps it as a proxy for "can't block."
pub struct NightbirdsClutches;

impl CardBehavior for NightbirdsClutches {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Nightbird's Clutches".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Up to two target creatures can't block this turn.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)])),
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

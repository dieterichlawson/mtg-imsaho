use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Giant Growth — {G} instant. Target creature gets +3/+3 until end of turn.
pub struct GiantGrowth;

impl CardBehavior for GiantGrowth {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Giant Growth".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target creature gets +3/+3 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.until_end_of_turn_effects.push(
                    crate::state::UntilEndOfTurnEffect {
                        target: *target_id,
                        power_mod: 3,
                        toughness_mod: 3,
                    }
                );
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

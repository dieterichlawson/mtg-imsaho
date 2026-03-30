use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, UntilEndOfTurnKeyword};
use crate::types::*;

/// Ranger's Guile — {G} instant. Target creature you control gets +1/+1 and gains hexproof until end of turn.
pub struct RangersGuile;

impl CardBehavior for RangersGuile {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ranger's Guile".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target creature you control gets +1/+1 and gains hexproof until end of turn.".into(),
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
                        power_mod: 1,
                        toughness_mod: 1,
                    }
                );
                state.until_end_of_turn_keywords.push(
                    UntilEndOfTurnKeyword {
                        target: *target_id,
                        keyword: Keyword::Hexproof,
                    }
                );
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{GameState, UntilEndOfTurnKeyword};
use crate::types::*;

/// Moment of Heroism — {1}{W} instant. Target creature gets +2/+2 and gains lifelink until end of turn.
pub struct MomentOfHeroism;

impl CardBehavior for MomentOfHeroism {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moment of Heroism".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target creature gets +2/+2 and gains lifelink until end of turn.".into(),
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
                state.until_end_of_turn_effects.push(
                    crate::state::UntilEndOfTurnEffect {
                        target: *target_id,
                        power_mod: 2,
                        toughness_mod: 2,
                    }
                );
                state.until_end_of_turn_keywords.push(
                    UntilEndOfTurnKeyword {
                        target: *target_id,
                        keyword: Keyword::Lifelink,
                    }
                );
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

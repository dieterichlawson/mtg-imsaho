use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Geistflame — {R} instant. Deal 1 damage to any target.
pub struct Geistflame;

impl CardBehavior for Geistflame {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Geistflame".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Geistflame deals 1 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::AnyTarget
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_damage(state, object_id, targets, 1);
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Lightning Bolt — {R} instant. Deal 3 damage to any target.
pub struct LightningBolt;

impl CardBehavior for LightningBolt {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Lightning Bolt".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Lightning Bolt deals 3 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::AnyTarget
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_damage(state, object_id, targets, 3, registry);
    }
}

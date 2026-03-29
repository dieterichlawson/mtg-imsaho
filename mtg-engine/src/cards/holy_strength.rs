use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
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

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }
}

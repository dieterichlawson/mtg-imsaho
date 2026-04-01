use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Gruesome Deformity — {B} aura enchantment. Enchanted creature has intimidate.
pub struct GruesomeDeformity;

impl CardBehavior for GruesomeDeformity {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Gruesome Deformity".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature has intimidate.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::Intimidate, scope: EffectScope::Attached },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }
}

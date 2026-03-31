use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Claustrophobia — {1}{U}{U} aura enchantment. When Claustrophobia enters the battlefield,
/// tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step.
pub struct Claustrophobia;

impl CardBehavior for Claustrophobia {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Claustrophobia".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "When Claustrophobia enters the battlefield, tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::PreventUntap { scope: EffectScope::Attached },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(target) = state.get_object_mut(*target_id) {
                target.tapped = true;
            }
        }
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }
}

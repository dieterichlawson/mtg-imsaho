use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Spectral Flight — {1}{U} aura enchantment. Enchanted creature gets +2/+2 and has flying.
pub struct SpectralFlight;

impl CardBehavior for SpectralFlight {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spectral Flight".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature gets +2/+2 and has flying.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 2, toughness: 2, scope: EffectScope::Attached },
                ContinuousEffect::GrantKeyword { keyword: Keyword::Flying, scope: EffectScope::Attached },
            ],
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }
}

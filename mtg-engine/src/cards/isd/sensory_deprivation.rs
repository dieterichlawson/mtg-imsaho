use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope};

/// Sensory Deprivation — {U} aura enchantment. Enchant creature. Enchanted creature gets -3/-0.
pub struct SensoryDeprivation;

impl CardBehavior for SensoryDeprivation {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sensory Deprivation".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nEnchanted creature gets -3/-0.".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: -3, toughness: 0, scope: EffectScope::Attached },
            ],
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets, registry);
    }
}

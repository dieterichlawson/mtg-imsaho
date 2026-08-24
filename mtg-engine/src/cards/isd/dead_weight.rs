use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope};

/// Dead Weight — {B} aura enchantment. Enchant creature. Enchanted creature gets -2/-2.
pub struct DeadWeight;

impl CardBehavior for DeadWeight {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Dead Weight".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nEnchanted creature gets -2/-2.".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: -2, toughness: -2, scope: EffectScope::Attached },
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

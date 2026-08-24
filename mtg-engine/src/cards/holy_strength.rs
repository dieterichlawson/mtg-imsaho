use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope};

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
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchanted creature gets +1/+2.".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 2, scope: EffectScope::Attached },
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

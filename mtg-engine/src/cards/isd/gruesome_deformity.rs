use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, Keyword, EffectScope};

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
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nEnchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)".into(),
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::Intimidate, scope: EffectScope::Attached },
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

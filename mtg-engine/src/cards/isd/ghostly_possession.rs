use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, Keyword, EffectScope};

/// Ghostly Possession — {2}{W} aura enchantment. Enchanted creature has flying.
/// Grants flying and prevents all combat damage to and from the enchanted creature.
pub struct GhostlyPossession;

impl CardBehavior for GhostlyPossession {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghostly Possession".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nEnchanted creature has flying.\nPrevent all combat damage that would be dealt to and dealt by enchanted creature.".into(),
            continuous_effects: vec![
                ContinuousEffect::GrantKeyword { keyword: Keyword::Flying, scope: EffectScope::Attached },
                ContinuousEffect::PreventCombatDamage { scope: EffectScope::Attached },
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

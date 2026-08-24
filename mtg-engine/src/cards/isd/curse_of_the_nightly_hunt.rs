use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope, CreatureFilter};

/// Curse of the Nightly Hunt — {2}{R} Enchantment — Aura Curse.
/// Enchant player.
/// Creatures enchanted player controls attack each combat if able.
pub struct CurseOfTheNightlyHunt;

impl CardBehavior for CurseOfTheNightlyHunt {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of the Nightly Hunt".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into(), "Curse".into()],
            oracle_text: "Enchant player\nCreatures enchanted player controls attack each combat if able.".into(),
            // Force attack on the cursed player's creatures.
            continuous_effects: vec![
                ContinuousEffect::ForceAttack {
                    scope: EffectScope::Global(CreatureFilter::ControlledByAttachedPlayer),
                },
            ],
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets, registry);
    }
}

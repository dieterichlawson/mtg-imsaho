use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Aura".into(), "Curse".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant player\nCreatures enchanted player controls attack each combat if able.".into(),
            keywords: vec![],
            flashback_cost: None,
            // ForceAttack on opponent's creatures (the cursed player).
            // Opponents filter matches creatures NOT controlled by the curse's controller
            // (i.e., the cursed opponent's creatures).
            continuous_effects: vec![
                ContinuousEffect::ForceAttack {
                    scope: EffectScope::Global(CreatureFilter::Opponents),
                },
            ],
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets);
    }
}

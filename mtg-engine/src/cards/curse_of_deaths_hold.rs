use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Curse of Death's Hold — {3}{B}{B} Enchantment — Aura Curse.
/// Enchant player.
/// Creatures enchanted player controls get -1/-1.
pub struct CurseOfDeathsHold;

impl CardBehavior for CurseOfDeathsHold {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of Death's Hold".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into(), "Curse".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant player\nCreatures enchanted player controls get -1/-1.".into(),
            keywords: vec![],
            flashback_cost: None,
            // Opponents filter: from the curse controller's perspective, the
            // enchanted player's creatures are "opponents'" creatures.
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: -1,
                    toughness: -1,
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

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Brimstone Volley — {2}{R} instant. Deal 3 damage to any target.
/// Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
pub struct BrimstoneVolley;

impl CardBehavior for BrimstoneVolley {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Brimstone Volley".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Brimstone Volley deals 3 damage to any target.\nMorbid — Brimstone Volley deals 5 damage instead if a creature died this turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::AnyTarget
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        let damage = if state.creature_died_this_turn { 5 } else { 3 };
        crate::cards::helpers::resolve_damage(state, object_id, targets, damage);
    }
}

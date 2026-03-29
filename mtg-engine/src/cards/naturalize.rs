use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Naturalize — {1}{G} instant. Destroy target artifact or enchantment.
pub struct Naturalize;

impl CardBehavior for Naturalize {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Naturalize".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target artifact or enchantment.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter("artifact or enchantment".into())
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                let registry = crate::cards::CardRegistry::with_all_cards();
                registry.card_data(obj.card_id)
                    .map(|d| d.card_types.contains(&CardType::Artifact) || d.card_types.contains(&CardType::Enchantment))
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}

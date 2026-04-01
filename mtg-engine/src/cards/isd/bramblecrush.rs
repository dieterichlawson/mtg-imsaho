use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Bramblecrush — {2}{G}{G} sorcery. Destroy target noncreature permanent.
pub struct Bramblecrush;

impl CardBehavior for Bramblecrush {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bramblecrush".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target noncreature permanent.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::Noncreature)
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                registry.card_data(obj.card_id)
                    .map(|d| !d.card_types.contains(&CardType::Creature))
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // "Destroy" always goes through the destruction pipeline,
        // which checks indestructible and regeneration.
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}

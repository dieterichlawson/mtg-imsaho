use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Urgent Exorcism — {1}{W} instant. Destroy target Spirit or enchantment.
pub struct UrgentExorcism;

impl CardBehavior for UrgentExorcism {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Urgent Exorcism".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target Spirit or enchantment.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::SubtypeOrCardType { subtypes: vec!["Spirit".into()], card_types: vec![CardType::Enchantment] })
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                let registry_match = registry.card_data(obj.card_id)
                    .map(|d| {
                        d.card_types.contains(&CardType::Enchantment)
                            || d.subtypes.contains(&"Spirit".to_string())
                    })
                    .unwrap_or(false);
                let obj_match = obj.card_types.contains(&CardType::Enchantment)
                    || obj.subtypes.iter().any(|s| s == "Spirit");
                registry_match || obj_match
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}

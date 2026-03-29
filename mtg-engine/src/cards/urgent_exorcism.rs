use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
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
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter("Spirit or enchantment".into())
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
                    .map(|d| {
                        d.card_types.contains(&CardType::Enchantment)
                            || d.subtypes.contains(&"Spirit".to_string())
                    })
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Battlefield {
                    state.move_object(*target_id, Zone::Graveyard);
                }
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

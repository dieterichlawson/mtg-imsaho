use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchanted creature has flying. Prevent all combat damage that would be dealt to and dealt by enchanted creature.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn granted_keywords(&self) -> Vec<Keyword> {
        vec![Keyword::Flying]
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.move_object(object_id, Zone::Battlefield);
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.attached_to = Some(*target_id);
                    obj.summoning_sick = false;
                }
                return;
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

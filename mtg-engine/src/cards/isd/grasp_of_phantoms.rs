use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Grasp of Phantoms — {3}{U} Sorcery.
/// Put target creature on top of its owner's library.
/// Flashback {7}{U}.
pub struct GraspOfPhantoms;

impl CardBehavior for GraspOfPhantoms {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grasp of Phantoms".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Put target creature on top of its owner's library.\nFlashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(7), ManaSymbol::Colored(Color::Blue)])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Battlefield {
                    let name = obj.name.clone();
                    state.put_into_library(*target_id, crate::state::LibraryPosition::Top, registry);
                    state.log(crate::state::LogLevel::Event,
                        format!("Grasp of Phantoms put {name} on top of its owner's library"));
                }
            }
        }
    }
}

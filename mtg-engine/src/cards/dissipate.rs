use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::*;

/// Dissipate — {1}{U}{U} instant. Counter target spell. Exile it instead of graveyard.
pub struct Dissipate;

impl CardBehavior for Dissipate {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Dissipate".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Stack)
                    .unwrap_or(false)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Stack {
                    let name = obj.name.clone();
                    state.stack.retain(|e| e.as_spell() != Some(*target_id));
                    state.move_object(*target_id, Zone::Exile);
                    state.log(LogLevel::Event, format!("{} was countered and exiled", name));
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}

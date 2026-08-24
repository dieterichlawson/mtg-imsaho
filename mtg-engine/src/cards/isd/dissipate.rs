use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| o.zone == Zone::Stack)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Stack {
                    let countered_name = state.obj_name(*target_id);
                    state.stack.retain(|e| e.as_spell() != Some(*target_id));
                    state.move_object(*target_id, Zone::Exile, registry);
                    state.log(LogLevel::Event, format!("{countered_name} was countered and exiled"));
                }
            }
        }
    }
}

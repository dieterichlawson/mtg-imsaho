use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, TargetFilter, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Cackling Counterpart — {1}{U}{U} Instant.
/// Create a token that's a copy of target creature you control.
/// Flashback {5}{U}{U}.
pub struct CacklingCounterpart;

impl CardBehavior for CacklingCounterpart {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Cackling Counterpart".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Create a token that's a copy of target creature you control.\nFlashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);
                let token_id = state.create_token_copy(*target_id, controller, registry);
                let name = state.get_object(token_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(crate::state::LogLevel::Event,
                    format!("Cackling Counterpart creates a token copy of {name}"));
            }
        }
    }
}

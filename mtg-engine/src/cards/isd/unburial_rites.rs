use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Unburial Rites — {4}{B} sorcery. Return target creature card from your graveyard to the battlefield.
pub struct UnburialRites;

impl CardBehavior for UnburialRites {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Unburial Rites".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Return target creature card from your graveyard to the battlefield.\nFlashback {3}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::GraveyardCreature
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // The target was chosen at cast time; use it directly.
        if let Some(Target::Object(id)) = targets.first() {
            let id = *id;
            let returned_name = state.obj_name(id);
            state.move_object(id, Zone::Battlefield, registry);
            state.log(LogLevel::Event, format!("{returned_name} returned to the battlefield"));
        }
        // If target is missing (fizzled), do nothing — just clean up.
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Return target creature card from your graveyard to the battlefield.\nFlashback {3}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::GraveyardCreature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // The target was chosen at cast time; use it directly.
        if let Some(Target::Object(id)) = targets.first() {
            let id = *id;
            let name = state.get_object(id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(id, Zone::Battlefield, registry);
            state.log(LogLevel::Event, format!("{} returned to the battlefield", name));
        }
        // If target is missing (fizzled), do nothing — just clean up.
        state.move_spell_after_resolve(object_id, registry);
    }
}

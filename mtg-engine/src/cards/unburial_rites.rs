use crate::actions::Target;
use crate::cards::{CardBehavior, CardData};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Unburial Rites — {4}{B} sorcery. Return target creature card from your graveyard to the battlefield.
/// Simplified: no targeting at engine level; finds the best creature in controller's graveyard.
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
            oracle_text: "Return target creature card from your graveyard to the battlefield.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])),
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target]) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        // Find best creature in graveyard (highest power).
        let target = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some() && o.id != object_id)
            .max_by_key(|o| o.power.unwrap_or(0))
            .map(|o| o.id);
        if let Some(target_id) = target {
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(target_id, Zone::Battlefield);
            state.log(crate::state::LogLevel::Event, format!("{} returned to the battlefield", name));
        }
        state.move_spell_after_resolve(object_id);
    }
}

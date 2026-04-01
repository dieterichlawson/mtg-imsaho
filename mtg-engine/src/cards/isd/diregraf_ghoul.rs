use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::actions::Target;
use crate::state::GameState;
use crate::types::*;

/// Diregraf Ghoul — 2/2 for {B}. Enters the battlefield tapped.
/// Note: "enters tapped" is a static/replacement ability, NOT a triggered ability.
pub struct DiregrafGhoul;

impl CardBehavior for DiregrafGhoul {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Diregraf Ghoul".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Diregraf Ghoul enters the battlefield tapped.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.tapped = true;
        }
    }
}

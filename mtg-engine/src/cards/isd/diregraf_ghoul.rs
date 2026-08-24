use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::actions::Target;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "This creature enters tapped.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.tapped = true;
        }
    }
}

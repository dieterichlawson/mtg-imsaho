use crate::actions::Target;
use crate::cards::{CardBehavior, CardData};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Divination — {2}{U} sorcery. Draw two cards.
pub struct Divination;

impl CardBehavior for Divination {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Divination".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Draw two cards.".into(),
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target]) {
        let controller = state.get_object(object_id)
            .map(|o| o.controller)
            .unwrap_or(crate::ids::PlayerId(0));

        crate::engine::draw_cards(state, controller, 2);
        state.move_object(object_id, Zone::Graveyard);
    }
}

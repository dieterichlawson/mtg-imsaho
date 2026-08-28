use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
            oracle_text: "Draw two cards.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        let _ = crate::engine::draw_cards(state, controller, 2, registry);
    }
}

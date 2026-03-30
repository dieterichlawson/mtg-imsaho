use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Moan of the Unhallowed — {2}{B}{B} sorcery. Create two 2/2 black Zombie tokens.
pub struct MoanOfTheUnhallowed;

impl CardBehavior for MoanOfTheUnhallowed {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moan of the Unhallowed".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Create two 2/2 black Zombie creature tokens.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Black), ManaSymbol::Colored(Color::Black)])),
            continuous_effects: vec![], triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        for _ in 0..2 {
            state.create_token("Zombie", controller, 2, 2, vec![Color::Black], vec![CardType::Creature], vec![]);
        }
        state.move_spell_after_resolve(object_id);
    }
}

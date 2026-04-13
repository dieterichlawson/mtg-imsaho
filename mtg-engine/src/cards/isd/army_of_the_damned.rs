use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Army of the Damned — {5}{B}{B}{B} sorcery.
/// Create thirteen tapped 2/2 black Zombie creature tokens.
/// Flashback {7}{B}{B}{B}.
pub struct ArmyOfTheDamned;

impl CardBehavior for ArmyOfTheDamned {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Army of the Damned".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Create thirteen tapped 2/2 black Zombie creature tokens.\nFlashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(7),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        for _ in 0..13 {
            let token_ids = state.create_token_with_subtypes(
                "Zombie", controller, 2, 2,
                vec![Color::Black],
                vec![CardType::Creature],
                vec![],
                vec!["Zombie".into()],
                registry,
            );
            // They enter the battlefield tapped.
            for token_id in token_ids {
                if let Some(obj) = state.get_object_mut(token_id) {
                    obj.tapped = true;
                }
            }
        }
        state.log(crate::state::LogLevel::Event,
            "Army of the Damned created 13 tapped Zombie tokens".to_string());
        state.move_spell_after_resolve(object_id, registry);
    }
}

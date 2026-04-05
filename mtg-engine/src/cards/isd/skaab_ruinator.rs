use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Skaab Ruinator — {1}{U}{U} 5/6 Zombie Horror with Flying.
/// As an additional cost to cast, exile three creature cards from your graveyard.
/// You may cast Skaab Ruinator from your graveyard.
pub struct SkaabRuinator;

impl CardBehavior for SkaabRuinator {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skaab Ruinator".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into(), "Horror".into()],
            power: Some(5),
            toughness: Some(6),
            oracle_text: "As an additional cost to cast this spell, exile three creature cards from your graveyard.\nFlying\nYou may cast this card from your graveyard.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(3)),
            triggered_abilities: vec![],
        }
    }

    fn can_cast_from_graveyard(&self) -> bool { true }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        // The exile of 3 creature cards happens at cast time (additional cost),
        // handled by the engine. On resolve, just enter the battlefield.
        state.move_object(object_id, Zone::Battlefield);
    }
}

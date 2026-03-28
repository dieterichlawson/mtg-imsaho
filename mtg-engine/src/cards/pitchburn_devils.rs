use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Pitchburn Devils — {4}{R} 3/3 Devil. When it dies, deal 3 damage to any target.
pub struct PitchburnDevils;

impl CardBehavior for PitchburnDevils {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Pitchburn Devils".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Devil".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "When Pitchburn Devils dies, it deals 3 damage to any target.".into(),
            keywords: vec![],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(controller);
        let old_life = state.get_player(opponent).life;
        let new_life = old_life - 3;
        state.get_player_mut(opponent).life = new_life;
        state.events.push(crate::events::GameEvent::LifeChanged { player: opponent, old: old_life, new_life });
        state.log(crate::state::LogLevel::Event, format!("Pitchburn Devils dealt 3 damage to p{}", opponent.0));
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Falkenrath Noble — {3}{B} 2/2 Vampire Noble. Flying.
/// Whenever a creature dies, target player loses 1 life and you gain 1 life.
pub struct FalkenrathNoble;

impl CardBehavior for FalkenrathNoble {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Falkenrath Noble".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into(), "Noble".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying\nWhenever a creature dies, target player loses 1 life and you gain 1 life.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        let opponent = state.opponent(controller);
        // Opponent loses 1 life.
        let old = state.get_player(opponent).life;
        state.get_player_mut(opponent).life = old - 1;
        state.events.push(crate::events::GameEvent::LifeChanged { player: opponent, old, new_life: old - 1 });
        // You gain 1 life.
        let old_self = state.get_player(controller).life;
        state.get_player_mut(controller).life = old_self + 1;
        state.events.push(crate::events::GameEvent::LifeChanged { player: controller, old: old_self, new_life: old_self + 1 });
    }
}

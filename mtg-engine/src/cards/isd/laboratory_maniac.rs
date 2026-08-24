use crate::ids::ObjectId;
use crate::state::GameState;
use crate::cards::{CardRegistry, CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Laboratory Maniac — {2}{U} 2/2 Human Wizard.
/// If you would draw a card while your library has no cards in it, you win the game instead.
pub struct LaboratoryManiac;

impl CardBehavior for LaboratoryManiac {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Laboratory Maniac".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "If you would draw a card while your library has no cards in it, you win the game instead.".into(),
            ..Default::default()
        }
    }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        _registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        use crate::replacement::{ReplaceableEvent, Replacement};
        let ReplaceableEvent::DrawsFromEmptyLibrary { player } = event else { return None };
        if state.get_object(self_id).map(|o| o.controller) != Some(*player) {
            return None;
        }
        // The draw does not happen; winning happens instead. Clearing the flag
        // stops the state-based action from killing them for it first.
        state.get_player_mut(*player).has_drawn_from_empty = false;
        let opponent = state.opponent(*player);
        state.players[opponent.0 as usize].lost = true;
        state.events.push(crate::events::GameEvent::PlayerLost {
            player: opponent,
            reason: crate::events::LossReason::LifeReachedZero,
        });
        state.result = Some(crate::state::GameResult::Winner(*player));
        let name = state.obj_name(self_id);
        state.log(crate::state::LogLevel::Milestone,
            format!("p{} wins the game with {name}!", player.0));
        Some(Replacement::Replaced)
    }
}

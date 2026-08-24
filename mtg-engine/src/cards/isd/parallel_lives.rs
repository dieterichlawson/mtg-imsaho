use crate::ids::ObjectId;
use crate::state::GameState;
use crate::cards::{CardRegistry, CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Parallel Lives — {3}{G} Enchantment.
/// If an effect would create one or more tokens under your control,
/// it creates twice that many of those tokens instead.
pub struct ParallelLives;

impl CardBehavior for ParallelLives {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Parallel Lives".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Enchantment],
            oracle_text: "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".into(),
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
        let ReplaceableEvent::CreatesTokens { controller, count } = event else { return None };
        if state.get_object(self_id).map(|o| o.controller) != Some(*controller) {
            return None;
        }
        Some(Replacement::Modified(ReplaceableEvent::CreatesTokens {
            controller: *controller,
            count: count * 2,
        }))
    }
}

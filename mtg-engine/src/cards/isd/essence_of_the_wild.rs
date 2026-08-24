use crate::ids::ObjectId;
use crate::state::GameState;
use crate::cards::{CardRegistry, CardBehavior, CardData};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Essence of the Wild {3}{G}{G}{G} 6/6 Avatar.
/// Creatures you control enter as a copy of Essence of the Wild.
///
/// This is a replacement effect (CR 614.1d): the creature never exists in its
/// original form on the battlefield. The engine checks for `EnterAsCopy` via
/// the card registry in `apply_entering_copy_replacement`.
pub struct EssenceOfTheWild;

impl CardBehavior for EssenceOfTheWild {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Essence of the Wild".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Avatar".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Creatures you control enter as a copy of this creature.".into(),
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
        let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
        // Not itself, only creatures, only ours, and only once.
        let (controller, card_id) = match state.get_object(self_id) {
            Some(o) => (o.controller, o.card_id),
            None => return None,
        };
        let is_creature = state.get_object(e.object).is_some_and(|o| o.power.is_some());
        if e.object == self_id || !is_creature || e.controller != controller || e.copy_of.is_some() {
            return None;
        }
        let mut e = e.clone();
        e.copy_of = Some(card_id);
        Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
    }
}

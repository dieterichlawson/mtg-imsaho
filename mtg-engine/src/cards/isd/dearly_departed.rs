use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone, CounterType};

/// Dearly Departed — {4}{W}{W} 5/5 Spirit with Flying.
/// As long as this creature is in your graveyard, each Human creature you control
/// enters with an additional +1/+1 counter on it.
pub struct DearlyDeparted;

impl CardBehavior for DearlyDeparted {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Dearly Departed".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Flying\nAs long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        }
    }

    /// This one works from the graveyard, not the battlefield.
    fn replacement_zones(&self) -> Vec<Zone> {
        vec![Zone::Graveyard]
    }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        use crate::replacement::{ReplaceableEvent, Replacement};
        let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
        // Must be in our graveyard, and only our own Humans get the counter.
        let owner = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Graveyard => o.owner,
            _ => return None,
        };
        if e.controller != owner || !state.has_subtype(e.object, "Human", registry) {
            return None;
        }
        let mut e = e.clone();
        e.counters.push((CounterType::PlusOnePlusOne, 1));
        Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
    }

}

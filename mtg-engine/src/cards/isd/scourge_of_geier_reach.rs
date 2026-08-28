use crate::cards::{CardRegistry, CardBehavior, CardData};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Scourge of Geier Reach — {3}{R}{R} 3/3 Elemental.
/// Scourge of Geier Reach gets +1/+1 for each creature your opponents control.
pub struct ScourgeOfGeierReach;

impl CardBehavior for ScourgeOfGeierReach {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Scourge of Geier Reach".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Elemental".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "This creature gets +1/+1 for each creature your opponents control.".into(),
            ..Default::default()
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Option<(i32, i32)> {
        let controller = state.get_object(object_id)?.controller;
        // "each creature your opponents control" — everyone who isn't you, not
        // one named opponent.
        let opponent_creatures = i32::try_from(state.all_objects_in_zone(Zone::Battlefield).into_iter()
            .filter(|o| o.controller != controller && state.is_creature(o.id, registry))
            .count()).unwrap_or(i32::MAX);
        // Base 3/3 + N/N where N = opponent creature count.
        Some((3 + opponent_creatures, 3 + opponent_creatures))
    }
}

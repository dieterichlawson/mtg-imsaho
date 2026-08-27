use crate::cards::{CardRegistry, CardBehavior, CardData};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Boneyard Wurm — {1}{G} */* Wurm.
/// Boneyard Wurm's power and toughness are each equal to the number of
/// creature cards in your graveyard.
pub struct BoneyardWurm;

impl CardBehavior for BoneyardWurm {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Boneyard Wurm".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Wurm".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.".into(),
            ..Default::default()
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Option<(i32, i32)> {
        let controller = state.get_object(object_id)?.controller;
        let creature_cards_in_gy = i32::try_from(state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| state.is_card(o.id) && state.is_creature(o.id, registry))
            .count()).unwrap_or(i32::MAX);
        Some((creature_cards_in_gy, creature_cards_in_gy))
    }
}

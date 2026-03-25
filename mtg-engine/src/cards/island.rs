use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

pub struct Island;

impl CardBehavior for Island {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Island".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![Supertype::Basic],
            subtypes: vec!["Island".into()],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {U}.".into(),
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Add {U}".into(),
                produced: vec![(ManaType::Blue, 1)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }
}

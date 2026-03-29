use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

pub struct Swamp;

impl CardBehavior for Swamp {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Swamp".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![Supertype::Basic],
            subtypes: vec!["Swamp".into()],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {B}.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![],
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
                description: "Add {B}".into(),
                produced: vec![(ManaType::Black, 1)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }
}

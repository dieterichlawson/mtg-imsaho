use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Supertype, Zone, ManaType};

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
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Add {U}".into(),
                produced: vec![(ManaType::Blue, 1)],
                requires_tap: true,
                has_side_effects: false,
            }]
        } else {
            vec![]
        }
    }
}

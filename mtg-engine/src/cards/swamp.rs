use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, CardType, Supertype, ManaType};

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
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {B}".into(),
            produced: vec![(ManaType::Black, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }
}

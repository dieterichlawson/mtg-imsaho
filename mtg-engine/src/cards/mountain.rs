use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, CardType, Supertype, ManaType};

pub struct Mountain;

impl CardBehavior for Mountain {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mountain".into(),
            card_types: vec![CardType::Land],
            supertypes: vec![Supertype::Basic],
            subtypes: vec!["Mountain".into()],
            oracle_text: "{T}: Add {R}.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {R}".into(),
            produced: vec![(ManaType::Red, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }
}

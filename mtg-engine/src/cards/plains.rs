use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, CardType, Supertype, ManaType};

pub struct Plains;

impl CardBehavior for Plains {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Plains".into(),
            card_types: vec![CardType::Land],
            supertypes: vec![Supertype::Basic],
            subtypes: vec!["Plains".into()],
            oracle_text: "{T}: Add {W}.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {W}".into(),
            produced: vec![(ManaType::White, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }
}

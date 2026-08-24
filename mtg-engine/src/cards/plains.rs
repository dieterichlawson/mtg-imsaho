use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, CardType, Supertype, ManaType};

pub struct Plains;

impl CardBehavior for Plains {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Plains".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![Supertype::Basic],
            subtypes: vec!["Plains".into()],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {W}.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
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

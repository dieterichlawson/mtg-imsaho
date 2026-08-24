use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ManaType};

/// Avacyn's Pilgrim — {G} 1/1 Human Monk.
/// {T}: Add {W}.
pub struct AvacynsPilgrim;

impl CardBehavior for AvacynsPilgrim {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Avacyn's Pilgrim".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Monk".into()],
            power: Some(1),
            toughness: Some(1),
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

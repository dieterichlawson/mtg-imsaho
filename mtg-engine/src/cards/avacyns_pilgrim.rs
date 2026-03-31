use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped && !obj.summoning_sick {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Add {W}".into(),
                produced: vec![(ManaType::White, 1)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }
}

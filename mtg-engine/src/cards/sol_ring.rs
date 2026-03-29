use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Sol Ring — {1} artifact. {T}: Add {C}{C}.
pub struct SolRing;

impl CardBehavior for SolRing {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sol Ring".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {C}{C}.".into(),
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
                description: "Add {C}{C}".into(),
                produced: vec![(ManaType::Colorless, 2)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }
}

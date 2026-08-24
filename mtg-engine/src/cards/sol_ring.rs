use crate::cards::{CardBehavior, CardData, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ManaType};

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
            oracle_text: "{T}: Add {C}{C}.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}{C}".into(),
            produced: vec![(ManaType::Colorless, 2)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }
}

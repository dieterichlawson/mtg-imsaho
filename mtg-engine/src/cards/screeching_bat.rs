use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Screeching Bat {2}{B} 2/2 Bat with Flying // Stalking Vampire 5/5 Vampire.
/// {2}{B}{B}: Transform Screeching Bat. (Activatable in either direction.)
pub struct ScreechingBat;

impl CardBehavior for ScreechingBat {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Screeching Bat".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Bat".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying\n{2}{B}{B}: Transform Screeching Bat. Activate only as a sorcery.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Stalking Vampire".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "{2}{B}{B}: Transform Stalking Vampire. Activate only as a sorcery.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        })
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
            Some((5, 5))
        } else {
            None
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        };
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{2}{B}{B}: Transform".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: true,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let is_transformed = state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_transformed = !is_transformed;
            let name = if obj.is_transformed { "Stalking Vampire" } else { "Screeching Bat" };
            obj.name = name.into();
            state.log(crate::state::LogLevel::Event,
                format!("{} transforms into {}", if is_transformed { "Stalking Vampire" } else { "Screeching Bat" }, name));
        }
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}

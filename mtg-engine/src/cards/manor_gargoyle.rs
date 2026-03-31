use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{GameState, UntilEndOfTurnKeyword};
use crate::types::*;

/// Manor Gargoyle — {5} 4/4 Artifact Creature — Gargoyle.
/// Defender, Indestructible.
/// {1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
///
/// Simplified: the activated ability grants flying until end of turn.
/// Losing defender is tracked by granting a special state.
pub struct ManorGargoyle;

impl CardBehavior for ManorGargoyle {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Manor Gargoyle".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
            ])),
            card_types: vec![CardType::Artifact, CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Gargoyle".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Defender, indestructible\n{1}: Until end of turn, Manor Gargoyle loses defender and gains flying.".into(),
            keywords: vec![Keyword::Defender, Keyword::Indestructible],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}: Loses defender, gains flying until end of turn".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        // Gain flying until end of turn.
        state.until_end_of_turn_keywords.push(UntilEndOfTurnKeyword {
            target: object_id,
            keyword: Keyword::Flying,
        });
        // Lose defender: remove it from the object's keywords.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.keywords.retain(|k| *k != Keyword::Defender);
        }
        state.log(crate::state::LogLevel::Event,
            "Manor Gargoyle loses defender and gains flying until end of turn".to_string());
    }
}

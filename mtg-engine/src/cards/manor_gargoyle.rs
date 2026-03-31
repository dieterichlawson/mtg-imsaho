use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{GameState, UntilEndOfTurnKeyword};
use crate::types::*;

/// Manor Gargoyle — {5} 4/4 Artifact Creature — Gargoyle.
/// Defender.
/// Manor Gargoyle has indestructible as long as it has defender.
/// {1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
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
            oracle_text: "Defender\nManor Gargoyle has indestructible as long as it has defender.\n{1}: Until end of turn, Manor Gargoyle loses defender and gains flying.".into(),
            keywords: vec![Keyword::Defender],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::ConditionalKeyword {
                    keyword: Keyword::Indestructible,
                    condition: EffectCondition::SelfHasKeyword(Keyword::Defender),
                    scope: EffectScope::OnSelf,
                },
            ],
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
        // Lose defender until end of turn (which also loses indestructible
        // since "has indestructible as long as it has defender").
        state.until_end_of_turn_removed_keywords.push(UntilEndOfTurnKeyword {
            target: object_id,
            keyword: Keyword::Defender,
        });
        state.log(crate::state::LogLevel::Event,
            "Manor Gargoyle loses defender and gains flying until end of turn".to_string());
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .map(|o| o.zone == Zone::Battlefield && o.power.is_some())
                .unwrap_or(false),
            _ => false,
        }
    }
}

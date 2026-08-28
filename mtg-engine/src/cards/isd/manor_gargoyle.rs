use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, CardType, Keyword, ContinuousEffect, EffectCondition, EffectScope, Zone};

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
            subtypes: vec!["Gargoyle".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Defender\nThis creature has indestructible as long as it has defender.\n{1}: Until end of turn, this creature loses defender and gains flying.".into(),
            keywords: vec![Keyword::Defender],
            continuous_effects: vec![
                ContinuousEffect::when(
                    EffectCondition::SelfHasKeyword(Keyword::Defender),
                    ContinuousEffect::GrantKeyword { keyword: Keyword::Indestructible, scope: EffectScope::OnSelf },
                ),
            ],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
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
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        // Gain flying until end of turn.
        state.until_end_of_turn.push(TemporaryEffect::GrantKeyword {
            target: object_id,
            keyword: Keyword::Flying,
        });
        // Lose defender until end of turn (which also loses indestructible
        // since "has indestructible as long as it has defender").
        state.until_end_of_turn.push(TemporaryEffect::RemoveKeyword {
            target: object_id,
            keyword: Keyword::Defender,
        });
        state.log(crate::state::LogLevel::Event,
            "Manor Gargoyle loses defender and gains flying until end of turn".to_string());
    }
}

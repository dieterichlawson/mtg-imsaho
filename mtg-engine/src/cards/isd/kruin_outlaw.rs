use crate::cards::helpers;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, ContinuousEffect, EffectScope, CreatureFilter};
use crate::actions::Target;

/// Kruin Outlaw {1}{R}{R} 2/2 Human Rogue Werewolf with First Strike
/// // Terror of Kruin Pass 3/3 Werewolf with Double Strike + menace
pub struct KruinOutlaw;


impl CardBehavior for KruinOutlaw {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kruin Outlaw".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Rogue".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "First strike\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            keywords: vec![Keyword::FirstStrike],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Terror of Kruin Pass".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Werewolf".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Double strike\nWerewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".into(),
            keywords: vec![Keyword::DoubleStrike],
            continuous_effects: vec![
                // "Werewolves you control have menace."
                ContinuousEffect::GrantKeyword {
                    keyword: Keyword::Menace,
                    scope: EffectScope::Global(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::HasSubtype("Werewolf".into()),
                    ])),
                },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        })
    }

    fn should_trigger(&self, state: &GameState, self_id: ObjectId, kind: &TriggerKind, registry: &CardRegistry) -> bool {
        helpers::werewolf_should_trigger(self, state, self_id, kind, registry)
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        helpers::werewolf_should_transform(state, object_id)
    }


    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        helpers::werewolf_on_upkeep(self, state, self_id, registry);
    }
}

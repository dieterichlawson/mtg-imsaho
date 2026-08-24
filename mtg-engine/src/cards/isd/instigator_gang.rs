use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef, helpers};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, CreatureFilter, EffectScope, Keyword, Zone};
use crate::actions::Target;

/// Instigator Gang {3}{R} 2/3 Human Werewolf — attacking creatures you control get +1/+0
/// // Wildblood Pack 5/5 Werewolf with Trample — attacking creatures you control get +3/+0
pub struct InstigatorGang;


impl CardBehavior for InstigatorGang {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Instigator Gang".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Attacking creatures you control get +1/+0.\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            // "Attacking creatures you control get +1/+0" is a static ability,
            // not a trigger. Modelling it as an AnyCreatureAttacks trigger that
            // pushed an until-end-of-turn buff diverged three ways: the buff
            // outlived the combat, creatures put onto the battlefield attacking
            // never got it (no attack was declared for them), and creatures
            // already attacking when the Gang arrived never got it either.
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 1,
                    toughness: 0,
                    scope: EffectScope::Global(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::Attacking,
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
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Wildblood Pack".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Werewolf".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Trample\nAttacking creatures you control get +3/+0.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.".into(),
            keywords: vec![Keyword::Trample],
            continuous_effects: vec![
                ContinuousEffect::ModifyPT {
                    power: 3,
                    toughness: 0,
                    scope: EffectScope::Global(CreatureFilter::And(vec![
                        CreatureFilter::ControlledByYou,
                        CreatureFilter::Attacking,
                    ])),
                },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform back if 2+ spells cast".into(),
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

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Option<(i32, i32)> {
        if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
            Some((5, 5))
        } else {
            None
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            let was_transformed = state.get_object(self_id).is_some_and(|o| o.is_transformed);
            helpers::apply_transform(state, self_id, registry);
            let (old_name, new_name) = if was_transformed {
                ("Wildblood Pack", "Instigator Gang")
            } else {
                ("Instigator Gang", "Wildblood Pack")
            };
            state.log(crate::state::LogLevel::Event,
                format!("{old_name} transforms into {new_name}"));
        }
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef, helpers};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
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
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Werewolf".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Attacking creatures you control get +1/+0.\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform".into(),
                target_requirement: None,
                },
                // Watch ALL creatures attacking (not just self).
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureAttacks,
                    description: "attacking creatures you control get +1/+0".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Wildblood Pack".into(),
            cost: None,
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Werewolf".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Trample\nAttacking creatures you control get +3/+0.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.".into(),
            keywords: vec![Keyword::Trample],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureAttacks,
                    description: "attacking creatures you control get +3/+0".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "transform back if 2+ spells cast".into(),
                target_requirement: None,
                },
            ],
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

    fn on_any_creature_attacks(&self, state: &mut GameState, self_id: ObjectId, attacker_id: ObjectId, attacker_controller: PlayerId, _registry: &CardRegistry) {
        let (controller, is_transformed) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
            _ => return,
        };
        // Only buff creatures you control.
        if attacker_controller != controller {
            return;
        }
        let bonus = if is_transformed { 3 } else { 1 };
        state.until_end_of_turn.push(
            crate::state::TemporaryEffect::ModifyPTWhileSourceInPlay {
                target: attacker_id,
                source: self_id,
                power_mod: bonus,
                toughness_mod: 0,
            }
        );
        let face = if is_transformed { "Wildblood Pack" } else { "Instigator Gang" };
        let name = state.get_object(attacker_id).map(|o| o.name.clone()).unwrap_or_default();
        state.log(crate::state::LogLevel::Event,
            format!("{face}: {name} gets +{bonus}/+0"));
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

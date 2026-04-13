use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef, helpers};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Instigator Gang {3}{R} 2/3 Human Werewolf — attacking creatures you control get +1/+0
/// // Wildblood Pack 5/5 Werewolf with Trample — attacking creatures you control get +3/+0
pub struct InstigatorGang;

impl InstigatorGang {
    fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
        let is_transformed = state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false);
        let total_spells_last_turn: u32 = state.num_spells_cast_last_turn.values().sum();
        if !is_transformed {
            total_spells_last_turn == 0 && !state.is_first_turn
        } else {
            state.num_spells_cast_last_turn.values().any(|&count| count >= 2)
        }
    }
}

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
                },
                // Watch ALL creatures attacking (not just self).
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureAttacks,
                    description: "attacking creatures you control get +1/+0".into(),
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
                },
            ],
        })
    }

    fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
        Self::werewolf_should_transform(state, object_id)
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
        if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
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
            format!("{}: {} gets +{}/+0", face, name, bonus));
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            return;
        }
        if self.should_transform(state, self_id, registry) {
            let was_transformed = state.get_object(self_id).map(|o| o.is_transformed).unwrap_or(false);
            helpers::apply_transform(state, self_id, registry);
            let (old_name, new_name) = if was_transformed {
                ("Wildblood Pack", "Instigator Gang")
            } else {
                ("Instigator Gang", "Wildblood Pack")
            };
            state.log(crate::state::LogLevel::Event,
                format!("{} transforms into {}", old_name, new_name));
        }
    }
}

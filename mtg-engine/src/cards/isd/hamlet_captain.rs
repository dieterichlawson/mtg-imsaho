use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Hamlet Captain — {1}{G} 2/2 Human Warrior.
/// Whenever this creature attacks or blocks, other Humans you control
/// get +1/+1 until end of turn.
pub struct HamletCaptain;

impl CardBehavior for HamletCaptain {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Hamlet Captain".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Warrior".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "other Humans get +1/+1 until end of turn".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::Blocks,
                    description: "other Humans get +1/+1 until end of turn".into(),
                },
            ],
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        self.buff_humans(state, self_id, registry);
    }

    fn on_blocks(&self, state: &mut GameState, self_id: ObjectId, _blocked_attacker: ObjectId, registry: &CardRegistry) {
        self.buff_humans(state, self_id, registry);
    }
}

impl HamletCaptain {
    fn buff_humans(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Find other Human creatures you control.
        let humans: Vec<ObjectId> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == controller
                    && o.power.is_some()
                    && o.id != self_id
            })
            .filter(|o| {
                o.subtypes.iter().any(|s| s == "Human")
                || registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                    .unwrap_or(false)
            })
            .map(|o| o.id)
            .collect();

        for id in &humans {
            state.until_end_of_turn_effects.push(
                crate::state::UntilEndOfTurnEffect {
                    target: *id,
                    power_mod: 1,
                    toughness_mod: 1,
                }
            );
        }

        if !humans.is_empty() {
            state.log(crate::state::LogLevel::Event,
                format!("Hamlet Captain: {} other Humans get +1/+1 until end of turn", humans.len()));
        }
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Elder Cathar — {2}{W} 2/2 Human Soldier.
/// When Elder Cathar dies, put a +1/+1 counter on target creature you control.
/// If that creature is a Human, put two +1/+1 counters on it instead.
pub struct ElderCathar;

impl CardBehavior for ElderCathar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Elder Cathar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "put +1/+1 counters on target creature".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        // Find creatures we control on the battlefield.
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == controller && o.power.is_some() && o.id != object_id)
            .map(|o| Target::Object(o.id))
            .collect();

        if targets.is_empty() {
            // No creatures to put counters on.
        } else if targets.len() == 1 {
            // Auto-add counters to the only creature.
            if let Target::Object(id) = targets[0] {
                let is_human = state.get_object(id)
                    .map(|o| {
                        let obj_has = o.subtypes.iter().any(|s| s == "Human");
                        let card_has = registry.card_data(o.card_id)
                            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                            .unwrap_or(false);
                        obj_has || card_has
                    })
                    .unwrap_or(false);
                let count = if is_human { 2 } else { 1 };
                state.add_counters(id, CounterType::PlusOnePlusOne, count);
                state.log(LogLevel::Event,
                    format!("Elder Cathar's death granted {} +1/+1 counter{}", count, if count > 1 { "s" } else { "" }));
            }
        } else {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Elder Cathar: put +1/+1 counter(s) on target creature you control".into(),
                    options: targets,
                    optional: false,
                    effect: PendingEffect::AddCounters { count: 1, human_bonus: true },
                },
            });
        }
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Crossway Vampire — 3/2 for {1}{R}{R}. Vampire.
/// When Crossway Vampire enters the battlefield, target creature can't block this turn.
pub struct CrosswayVampire;

impl CardBehavior for CrosswayVampire {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Crossway Vampire".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "When Crossway Vampire enters the battlefield, target creature can't block this turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "target creature can't block this turn".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Can target any creature (though in practice you'd target an opponent's creature).
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
            .map(|o| Target::Object(o.id))
            .collect();

        if targets.is_empty() {
            // No valid targets, do nothing.
        } else if targets.len() == 1 {
            // Auto-apply to the only target.
            if let Target::Object(target_id) = targets[0] {
                state.until_end_of_turn_cant_block.push(target_id);
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(LogLevel::Event,
                    format!("{} can't block this turn (Crossway Vampire)", name));
            }
        } else {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Crossway Vampire: target creature can't block this turn".into(),
                    options: targets,
                    optional: false,
                    effect: PendingEffect::CantBlockThisTurn { source_name: "Crossway Vampire".into() },
                },
            });
        }
    }
}

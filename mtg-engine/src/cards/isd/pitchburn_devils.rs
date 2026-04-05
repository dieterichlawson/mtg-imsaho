use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Pitchburn Devils — {4}{R} 3/3 Devil. When it dies, deal 3 damage to any target.
pub struct PitchburnDevils;

impl CardBehavior for PitchburnDevils {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Pitchburn Devils".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Devil".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "When this creature dies, it deals 3 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "deal 3 damage to any target".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Any target" — all creatures + all players.
        let targets = crate::cards::helpers::any_targets(state);
        crate::cards::helpers::present_target_choice(
            state, object_id, controller, targets,
            PendingEffect::DealDamage { amount: 3, source_id: object_id, source_name: "Pitchburn Devils".into() },
            "Pitchburn Devils: deal 3 damage to any target",
            false, // mandatory
        );
    }
}

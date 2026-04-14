use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

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
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::AnyTarget),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::DealDamage {
            amount: 3,
            source_id: object_id,
            source_name: "Pitchburn Devils".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

/// Crossway Vampire — 3/2 for {1}{R}{R}. Vampire.
/// When this creature enters, target creature can't block this turn.
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
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "When this creature enters, target creature can't block this turn.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "target creature can't block this turn".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::Creature),
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, _object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::CantBlockThisTurn {
            source_name: "Crossway Vampire".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}

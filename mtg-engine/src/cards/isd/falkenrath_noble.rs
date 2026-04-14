use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Falkenrath Noble — {3}{B} 2/2 Vampire Noble. Flying.
/// Whenever this creature or another creature dies, target player loses 1 life
/// and you gain 1 life.
pub struct FalkenrathNoble;

impl CardBehavior for FalkenrathNoble {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Falkenrath Noble".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into(), "Noble".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "target player loses 1 life, you gain 1 life".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "target player loses 1 life, you gain 1 life".into(),
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // "This creature dies" — trigger fires even when Noble itself dies.
        // Use controller (last known information from when it was on the battlefield).
        let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);
        drain(state, controller, chosen_targets, registry);
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, chosen_targets: &[Target], registry: &CardRegistry) {
        // "Another creature dies" — triggers on ANY creature death (any controller).
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            _ => return,
        };
        drain(state, controller, chosen_targets, registry);
    }
}

/// Apply Falkenrath Noble's drain to the chosen target player.
fn drain(state: &mut GameState, controller: PlayerId, chosen_targets: &[Target], registry: &CardRegistry) {
    // CR 603.3d: target was chosen when the trigger went on the stack.
    let Some(target) = chosen_targets.first() else { return };
    let effect = PendingEffect::DrainLife {
        controller,
        source_name: "Falkenrath Noble".into(),
    };
    crate::engine::apply_pending_effect(state, target, &effect, registry);
}

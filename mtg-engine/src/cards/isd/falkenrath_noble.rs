use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
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
            subtypes: vec!["Vampire".into(), "Noble".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life.".into(),
            keywords: vec![Keyword::Flying],
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
            ..Default::default()
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // "This creature dies" — trigger fires even when Noble itself dies.
        // Use controller (last known information from when it was on the battlefield).
        let controller = crate::cards::helpers::controller_of(state, object_id);
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

/// "...target player loses 1 life and you gain 1 life." Both amounts are this
/// card's text, so the effect is applied here rather than through a shared
/// engine effect. This was never a deferred resolution — the target is already
/// chosen (CR 603.3d, locked in when the trigger went on the stack) — so it
/// runs directly instead of round-tripping through `apply_pending_effect`.
fn drain(state: &mut GameState, controller: PlayerId, chosen_targets: &[Target], _registry: &CardRegistry) {
    let Some(Target::Player(pid)) = chosen_targets.first() else { return };
    state.lose_life(*pid, 1);
    state.gain_life(controller, 1);
    state.log(crate::state::LogLevel::Event,
        format!("Falkenrath Noble: p{} lost 1 life, p{} gained 1 life", pid.0, controller.0));
}

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "target player loses 1 life, you gain 1 life".into(),
                },
            ],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        // "This creature dies" — trigger fires even when Noble itself dies.
        // Use controller (last known information from when it was on the battlefield).
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        drain(state, controller, object_id);
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {
        // "Another creature dies" — triggers on ANY creature death (any controller).
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            _ => return,
        };
        drain(state, controller, self_id);
    }
}

/// Present a "target player" choice for Falkenrath Noble's drain effect.
fn drain(state: &mut GameState, controller: PlayerId, source_id: ObjectId) {
    use crate::actions::Target;
    use crate::state::PendingEffect;

    let targets: Vec<Target> = state.players.iter().map(|p| Target::Player(p.id)).collect();
    let effect = PendingEffect::DrainLife {
        controller,
        source_name: "Falkenrath Noble".into(),
    };

    crate::cards::helpers::present_target_choice(
        state, source_id, controller, targets, effect,
        "Falkenrath Noble: choose target player to lose 1 life",
        false, // mandatory
    );
}

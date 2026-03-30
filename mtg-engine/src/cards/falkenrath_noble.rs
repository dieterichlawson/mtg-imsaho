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
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
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
        // Use the owner since the Noble is already in the graveyard.
        let controller = state.get_object(object_id).map(|o| o.owner).unwrap_or(PlayerId(0));
        drain(state, controller);
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _registry: &CardRegistry) {
        // "Another creature dies" — triggers on ANY creature death (any controller).
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        drain(state, controller);
    }
}

/// Apply Falkenrath Noble's drain effect: opponent loses 1, you gain 1.
/// In 2-player, auto-targets the opponent (per project convention).
fn drain(state: &mut GameState, controller: PlayerId) {
    let opponent = state.opponent(controller);
    // Target player (opponent in 2-player) loses 1 life.
    let old = state.get_player(opponent).life;
    state.get_player_mut(opponent).life = old - 1;
    state.events.push(crate::events::GameEvent::LifeChanged { player: opponent, old, new_life: old - 1 });
    // You gain 1 life.
    let old_self = state.get_player(controller).life;
    state.get_player_mut(controller).life = old_self + 1;
    state.events.push(crate::events::GameEvent::LifeChanged { player: controller, old: old_self, new_life: old_self + 1 });
}

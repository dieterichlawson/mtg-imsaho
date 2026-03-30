use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Rage Thrower — {5}{R} 4/2 Human Shaman.
/// Whenever another creature dies, Rage Thrower deals 2 damage to target player or planeswalker.
pub struct RageThrower;

impl CardBehavior for RageThrower {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rage Thrower".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(4),
            toughness: Some(2),
            oracle_text: "Whenever another creature dies, Rage Thrower deals 2 damage to target player or planeswalker.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "deal 2 damage to target player".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // "Deals 2 damage to target player" — auto-target opponent in 2-player.
        // Uses damage (not life loss) so it interacts correctly with damage prevention.
        let opponent = state.opponent(controller);
        let old = state.get_player(opponent).life;
        let new_life = old - 2;
        state.get_player_mut(opponent).life = new_life;
        state.events.push(GameEvent::CombatDamageDealt {
            source: self_id,
            target: DamageTarget::Player(opponent),
            amount: 2,
        });
        state.events.push(GameEvent::LifeChanged { player: opponent, old, new_life });
        state.log(crate::state::LogLevel::Event, format!("Rage Thrower dealt 2 damage to p{}", opponent.0));
    }
}

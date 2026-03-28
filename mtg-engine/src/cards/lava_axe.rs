use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::events::{GameEvent, DamageTarget};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Lava Axe — {4}{R} sorcery. Deal 5 damage to target player.
pub struct LavaAxe;

impl CardBehavior for LavaAxe {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Lava Axe".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Lava Axe deals 5 damage to target player or planeswalker.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Player(player_id)) = targets.first() {
            let old_life = state.get_player(*player_id).life;
            let new_life = old_life - 5;
            state.get_player_mut(*player_id).life = new_life;
            state.events.push(GameEvent::CombatDamageDealt {
                source: object_id,
                target: DamageTarget::Player(*player_id),
                amount: 5,
            });
            state.events.push(GameEvent::LifeChanged {
                player: *player_id,
                old: old_life,
                new_life,
            });
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Swords to Plowshares — {W} instant. Exile target creature.
/// Its controller gains life equal to its power.
pub struct SwordsToPlowshares;

impl CardBehavior for SwordsToPlowshares {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Swords to Plowshares".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Exile target creature. Its controller gains life equal to its power.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Battlefield {
                    let controller = obj.controller;
                    let power = obj.power.unwrap_or(0).max(0) as i32;

                    // Exile the creature.
                    state.move_object(*target_id, Zone::Exile);

                    // Controller gains life equal to its power.
                    if power > 0 {
                        let old_life = state.get_player(controller).life;
                        let new_life = old_life + power;
                        state.get_player_mut(controller).life = new_life;
                        state.events.push(GameEvent::LifeChanged {
                            player: controller,
                            old: old_life,
                            new_life,
                        });
                    }
                }
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

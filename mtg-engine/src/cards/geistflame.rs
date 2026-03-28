use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::events::{GameEvent, DamageTarget};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Geistflame — {R} instant. Deal 1 damage to any target.
pub struct Geistflame;

impl CardBehavior for Geistflame {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Geistflame".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Geistflame deals 1 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)])),
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::AnyTarget
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(target) = targets.first() {
            match target {
                Target::Object(target_id) => {
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        if obj.zone == Zone::Battlefield {
                            obj.damage_marked += 1;
                            state.events.push(GameEvent::CombatDamageDealt {
                                source: object_id,
                                target: DamageTarget::Object(*target_id),
                                amount: 1,
                            });
                        }
                    }
                }
                Target::Player(player_id) => {
                    let old_life = state.get_player(*player_id).life;
                    let new_life = old_life - 1;
                    state.get_player_mut(*player_id).life = new_life;
                    state.events.push(GameEvent::CombatDamageDealt {
                        source: object_id,
                        target: DamageTarget::Player(*player_id),
                        amount: 1,
                    });
                    state.events.push(GameEvent::LifeChanged {
                        player: *player_id,
                        old: old_life,
                        new_life,
                    });
                }
            }
        }
        // Instant goes to graveyard after resolution.
        state.move_spell_after_resolve(object_id);
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Battlefield {
                    let controller = obj.controller;
                    // Use effective power (accounts for buffs/debuffs/counters).
                    let power = state.effective_power(*target_id, registry).unwrap_or(0).max(0);

                    // Exile the creature.
                    state.move_object(*target_id, Zone::Exile, registry);

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
    }
}

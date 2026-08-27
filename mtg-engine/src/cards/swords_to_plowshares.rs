use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
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
            oracle_text: "Exile target creature. Its controller gains life equal to its power.".into(),
            ..Default::default()
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
                        state.change_life(controller, power);
                    }
                }
            }
        }
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Giant Growth — {G} instant. Target creature gets +3/+3 until end of turn.
pub struct GiantGrowth;

impl CardBehavior for GiantGrowth {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Giant Growth".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Target creature gets +3/+3 until end of turn.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: *target_id,
                        power_mod: 3,
                        toughness_mod: 3,
                    }
                );
            }
        }
    }
}

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Smite the Monstrous — {3}{W} instant. Destroy target creature with power 4 or greater.
pub struct SmiteTheMonstrous;

impl CardBehavior for SmiteTheMonstrous {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Smite the Monstrous".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target creature with power 4 or greater.".into(),
            keywords: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter("power 4+".into())
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| {
                        o.zone == Zone::Battlefield
                            && o.power.is_some()
                            && o.power.unwrap_or(0) >= 4
                    })
                    .unwrap_or(false)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Battlefield {
                    state.move_object(*target_id, Zone::Graveyard);
                }
            }
        }
        state.move_object(object_id, Zone::Graveyard);
    }
}

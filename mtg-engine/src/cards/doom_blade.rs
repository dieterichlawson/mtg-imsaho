use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Doom Blade — {1}{B} instant. Destroy target nonblack creature.
pub struct DoomBlade;

impl CardBehavior for DoomBlade {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Doom Blade".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target nonblack creature.".into(),
            keywords: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter("nonblack".into())
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| {
                        o.zone == Zone::Battlefield
                            && o.power.is_some()
                            && !o.colors.contains(&Color::Black)
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

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Rebuke — {2}{W} instant. Destroy target attacking creature.
pub struct Rebuke;

impl CardBehavior for Rebuke {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rebuke".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target attacking creature.".into(),
            keywords: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter("attacking".into())
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) => o,
                    None => return false,
                };
                if obj.zone != Zone::Battlefield || obj.power.is_none() { return false; }
                let is_attacking = state.combat.as_ref()
                    .map(|c| c.attackers.contains_key(id))
                    .unwrap_or(false);
                is_attacking
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

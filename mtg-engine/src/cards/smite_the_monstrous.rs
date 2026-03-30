use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
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
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::PowerAtLeast(4))
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield && o.power.is_some() => o,
                    _ => return false,
                };
                // Use effective power (accounts for buffs/debuffs/counters).
                state.effective_power(obj.id, registry).unwrap_or(0) >= 4
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}

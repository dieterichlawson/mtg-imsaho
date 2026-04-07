use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Prey Upon — {G} sorcery. Target creature you control fights target creature you don't control.
pub struct PreyUpon;

impl CardBehavior for PreyUpon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Prey Upon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
            Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouDontControl)),
        )
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if targets.len() == 2 {
            if let (Target::Object(a), Target::Object(b)) = (&targets[0], &targets[1]) {
                // Fight requires both creatures on the battlefield.
                let a_on_bf = state.get_object(*a).map(|o| o.zone == Zone::Battlefield).unwrap_or(false);
                let b_on_bf = state.get_object(*b).map(|o| o.zone == Zone::Battlefield).unwrap_or(false);
                if a_on_bf && b_on_bf {
                    let caster = state.get_object(object_id).map(|o| o.controller);
                    let a_mine = caster.and_then(|c| state.get_object(*a).map(|o| o.controller == c)).unwrap_or(false);
                    let (my_creature, their_creature) = if a_mine {
                        (*a, *b)
                    } else {
                        (*b, *a)
                    };
                    crate::combat::fight(state, my_creature, their_creature, registry);
                }
            }
        }
        state.move_spell_after_resolve(object_id, registry);
    }
}
